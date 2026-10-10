//! Queue-owned, bounded PNG cache. Never scans or deletes outside its own namespace.
use image::{DynamicImage, RgbaImage};
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

const DIRECTORY: &str = "queue-thumbnails-v1";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

fn digest(value: &str) -> String {
    super::sha256::sha256_hex_bytes(value.as_bytes())[..32].to_owned()
}

#[derive(Default)]
struct Ownership {
    live: BTreeSet<String>,
    cleanup_allowed: bool,
    revision: u64,
    cleanup_running: bool,
}

pub(crate) struct QueueThumbnailCache {
    root: PathBuf,
    ownership: Mutex<Ownership>,
    io: Mutex<()>,
}

#[derive(Clone)]
pub(crate) struct QueueThumbnailCacheRequest {
    pub cache: Arc<QueueThumbnailCache>,
    pub owner: String,
    pub source_hash: Option<String>,
    pub bounds: [u32; 2],
}

impl QueueThumbnailCacheRequest {
    pub(crate) fn new(
        cache: Arc<QueueThumbnailCache>,
        owner: String,
        primary_url: Option<&str>,
        bounds: [u32; 2],
    ) -> Self {
        Self {
            cache,
            owner,
            source_hash: primary_url.filter(|url| !url.is_empty()).map(digest),
            bounds,
        }
    }
}

impl QueueThumbnailCache {
    pub(crate) fn new(cache_root: &Path) -> Arc<Self> {
        Arc::new(Self {
            root: cache_root.join(DIRECTORY),
            ownership: Mutex::new(Ownership::default()),
            io: Mutex::new(()),
        })
    }

    pub(crate) fn uses_root(&self, cache_root: &Path) -> bool {
        self.root == cache_root.join(DIRECTORY)
    }

    pub(crate) fn has_owner(&self, owner: &str) -> bool {
        self.ownership.lock().unwrap().live.contains(owner)
    }

    // UI updates only a short ownership lock; disk operations use a separate worker lock.
    pub(crate) fn sync_owners(self: &Arc<Self>, live: BTreeSet<String>, cleanup_allowed: bool) {
        let mut state = self.ownership.lock().unwrap();
        if state.live == live && state.cleanup_allowed == cleanup_allowed && state.revision != 0 {
            return;
        }
        state.live = live;
        state.cleanup_allowed = cleanup_allowed;
        state.revision += 1;
        if state.cleanup_running || !cleanup_allowed {
            return;
        }
        state.cleanup_running = true;
        let cache = Arc::clone(self);
        std::thread::spawn(move || {
            loop {
                let revision = cache.ownership.lock().unwrap().revision;
                if let Err(error) = cache.prune() {
                    eprintln!("[thumbnail-cache] {error}");
                }
                let mut state = cache.ownership.lock().unwrap();
                if state.revision == revision {
                    state.cleanup_running = false;
                    break;
                }
            }
        });
    }

    pub(crate) fn load(
        &self,
        request: &QueueThumbnailCacheRequest,
    ) -> Result<Option<DynamicImage>, String> {
        self.read_pixels(request, false)
    }

    pub(crate) fn load_fallback(
        &self,
        request: &QueueThumbnailCacheRequest,
    ) -> Result<Option<DynamicImage>, String> {
        self.read_pixels(request, true)
    }

    fn read_pixels(
        &self,
        request: &QueueThumbnailCacheRequest,
        allow_smaller: bool,
    ) -> Result<Option<DynamicImage>, String> {
        let _guard = self.io.lock().unwrap();
        let mut files = self.files()?;
        files.retain(|(_, name)| {
            name.owner == request.owner
                && request
                    .source_hash
                    .as_ref()
                    .is_none_or(|hash| *hash == name.source_hash)
        });
        // Missing URLs after safe queue restore can still use the newest cached image.
        files.sort_by_key(|(path, _)| fs::metadata(path).and_then(|meta| meta.modified()).ok());
        if request.source_hash.is_none() {
            if let Some((_, latest)) = files.last() {
                let hash = latest.source_hash.clone();
                files.retain(|(_, name)| name.source_hash == hash);
            }
        }
        files.sort_by_key(|(_, name)| u64::from(name.bounds[0]) * u64::from(name.bounds[1]));
        let adequate = |name: &CacheName| {
            name.bounds[0] >= request.bounds[0] && name.bounds[1] >= request.bounds[1]
        };
        let mut candidates: Vec<_> = files.iter().filter(|(_, name)| adequate(name)).collect();
        if request.source_hash.is_none() || allow_smaller {
            candidates.extend(files.iter().rev().filter(|(_, name)| !adequate(name)));
        }
        for (path, name) in candidates {
            let metadata =
                fs::symlink_metadata(path).map_err(|_| "Could not inspect thumbnail cache")?;
            if metadata.len() > 16 * 1024 * 1024 {
                continue;
            }
            let reader = match image::ImageReader::open(path)
                .and_then(|reader| reader.with_guessed_format())
            {
                Ok(reader) => reader,
                Err(_) => continue,
            };
            let Ok((width, height)) = reader.into_dimensions() else {
                continue;
            };
            if width == 0
                || height == 0
                || width > name.bounds[0]
                || height > name.bounds[1]
                || u64::from(width) * u64::from(height) > 16 * 1024 * 1024
            {
                continue;
            }
            if let Ok(image) = image::open(path) {
                return Ok(Some(image));
            }
        }
        Ok(None)
    }

    pub(crate) fn store(
        &self,
        request: &QueueThumbnailCacheRequest,
        pixels: &RgbaImage,
    ) -> Result<(), String> {
        let Some(hash) = &request.source_hash else {
            return Ok(());
        };
        let name = CacheName {
            owner: request.owner.clone(),
            source_hash: hash.clone(),
            bounds: request.bounds,
        };
        if CacheName::parse(&name.filename()).is_none() {
            return Err("Invalid thumbnail cache identity or bounds".into());
        }
        let _guard = self.io.lock().unwrap();
        if !self.ownership.lock().unwrap().live.contains(&request.owner) {
            return Ok(());
        }
        if pixels.width() > request.bounds[0] || pixels.height() > request.bounds[1] {
            return Err("Thumbnail exceeds requested pixel bounds".into());
        }
        self.ensure_root()?;
        let name = CacheName {
            owner: request.owner.clone(),
            source_hash: hash.clone(),
            bounds: request.bounds,
        };
        let destination = self.root.join(name.filename());
        if fs::symlink_metadata(&destination).is_ok_and(|meta| is_link(&meta)) {
            return Err("Refusing linked thumbnail cache entry".into());
        }
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = self.root.join(format!(
            "{}.{}-{sequence}.tmp",
            name.filename(),
            std::process::id()
        ));
        let mut created = false;
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|_| "Could not create thumbnail cache temporary file")?;
            created = true;
            pixels
                .write_to(&mut file, image::ImageFormat::Png)
                .map_err(|_| "Could not encode thumbnail cache")?;
            file.sync_all()
                .map_err(|_| "Could not sync thumbnail cache")?;
            drop(file);
            publish(&temporary, &destination).map_err(|_| "Could not publish thumbnail cache")?;
            // Late workers must not resurrect a removed queue item's cache.
            if !self.ownership.lock().unwrap().live.contains(&request.owner) {
                fs::remove_file(&destination)
                    .map_err(|_| "Could not remove retired thumbnail cache")?;
            } else if self.ownership.lock().unwrap().cleanup_allowed {
                for (path, old) in self.files()? {
                    if old.owner == request.owner && old.source_hash != *hash {
                        fs::remove_file(path)
                            .map_err(|_| "Could not remove superseded thumbnail cache")?;
                    }
                }
            }
            Ok(())
        })();
        // Only remove the exact temporary file created by this operation.
        if created {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    pub(crate) fn prune(&self) -> Result<(), String> {
        let _guard = self.io.lock().unwrap();
        let state = self.ownership.lock().unwrap();
        if !state.cleanup_allowed {
            return Ok(());
        }
        let live = state.live.clone();
        drop(state);
        for (path, name) in self.files()? {
            // Recheck current ownership before each deletion: additions may arrive mid-scan.
            if !live.contains(&name.owner) {
                let current = self.ownership.lock().unwrap();
                if current.cleanup_allowed && !current.live.contains(&name.owner) {
                    fs::remove_file(path)
                        .map_err(|_| "Could not clean unreferenced thumbnail cache")?;
                }
            }
        }
        Ok(())
    }

    fn ensure_root(&self) -> Result<(), String> {
        if !self.root.is_absolute() {
            return Err("Thumbnail cache root must be absolute".into());
        }
        fs::create_dir_all(&self.root).map_err(|_| "Could not create thumbnail cache directory")?;
        self.check_root().map(|_| ())
    }

    fn check_root(&self) -> Result<bool, String> {
        if !self.root.is_absolute() {
            return Err("Thumbnail cache root must be absolute".into());
        }
        match fs::symlink_metadata(&self.root) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Ok(meta) if meta.is_dir() && !is_link(&meta) => Ok(true),
            _ => Err("Refusing non-directory or linked thumbnail cache root".into()),
        }
    }

    fn files(&self) -> Result<Vec<(PathBuf, CacheName)>, String> {
        if !self.check_root()? {
            return Ok(Vec::new());
        }
        let entries = fs::read_dir(&self.root).map_err(|_| "Could not list thumbnail cache")?;
        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|_| "Could not inspect thumbnail cache entry")?;
            let meta = fs::symlink_metadata(entry.path())
                .map_err(|_| "Could not inspect thumbnail cache entry")?;
            if meta.is_file() && !is_link(&meta) {
                if let Some(name) = entry.file_name().to_str().and_then(CacheName::parse) {
                    files.push((entry.path(), name));
                }
            }
        }
        Ok(files)
    }
}

struct CacheName {
    owner: String,
    source_hash: String,
    bounds: [u32; 2],
}
impl CacheName {
    fn filename(&self) -> String {
        format!(
            "q-{}-{}-{}x{}.png",
            self.owner, self.source_hash, self.bounds[0], self.bounds[1]
        )
    }
    fn parse(name: &str) -> Option<Self> {
        let mut parts = name.strip_prefix("q-")?.strip_suffix(".png")?.split('-');
        let owner = parts.next()?;
        let hash = parts.next()?;
        let (width, height) = parts.next()?.split_once('x')?;
        if parts.next().is_some()
            || crate::domain::PersistentItemId::parse(owner.to_owned()).is_err()
            || [hash].iter().any(|value| {
                value.len() != 32
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            })
        {
            return None;
        }
        let bounds = [width.parse::<u32>().ok()?, height.parse::<u32>().ok()?];
        if bounds.contains(&0) {
            return None;
        }
        Some(Self {
            owner: owner.to_owned(),
            source_hash: hash.to_owned(),
            bounds,
        })
    }
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

#[cfg(not(windows))]
fn publish(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(source, target)
}
#[cfg(windows)]
fn publish(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<_> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(source: *const u16, target: *const u16, flags: u32) -> i32;
    }
    let success = unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), 0x1 | 0x8) };
    if success == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
