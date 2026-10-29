//! Private work, cross-process basename reservations, and non-replacing publication.
use super::{
    output_policy::{ExistingFilePolicy, safe_stem},
    storage_paths,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Duration,
};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn unique_id() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(format!(
        "{}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}

#[derive(Debug)]
pub enum OutputOutcome {
    Published(Vec<PathBuf>),
    SkippedExisting(PathBuf),
}

#[derive(Clone, Debug)]
pub struct OutputFailure {
    pub disposition: super::output_policy::OutputDisposition,
    pub published_paths: Vec<PathBuf>,
    pub message: String,
}

impl From<String> for OutputFailure {
    fn from(message: String) -> Self {
        Self {
            disposition: super::output_policy::OutputDisposition::Failed,
            published_paths: Vec::new(),
            message,
        }
    }
}
impl From<&str> for OutputFailure {
    fn from(message: &str) -> Self {
        message.to_owned().into()
    }
}
impl std::fmt::Display for OutputFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if !self.published_paths.is_empty() {
            write!(f, ". Complete published files: {:?}", self.published_paths)?;
        }
        Ok(())
    }
}
impl From<OutputFailure> for String {
    fn from(error: OutputFailure) -> Self {
        error.to_string()
    }
}

/// Candidate final files in one requested group, independent of its source input.
#[derive(Clone)]
pub struct OutputArtifacts {
    suffixes: Vec<String>,
    directory: bool,
    sections: bool,
}
impl OutputArtifacts {
    pub fn file(extension: &str) -> Self {
        Self {
            suffixes: vec![format!(".{}", extension.trim_start_matches('.'))],
            directory: false,
            sections: false,
        }
    }
    pub fn directory() -> Self {
        Self {
            suffixes: Vec::new(),
            directory: true,
            sections: false,
        }
    }
    pub fn with_suffix(mut self, suffix: &str) -> Self {
        self.suffixes
            .push(format!(".{}", suffix.trim_start_matches('.')));
        self
    }
    pub fn sections(mut self) -> Self {
        self.sections = true;
        self
    }
}

pub struct OutputTransaction {
    pub(crate) output_config: Option<std::sync::Arc<super::tools::EffectiveYtDlpConfig>>,
    status: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    pub(crate) plan: Option<super::tools::OutputPlan>,
    #[cfg(test)]
    pub(crate) after_publish: Option<Box<dyn FnMut(&Path) + Send>>,
    pub work: PathBuf,
    pub destination: PathBuf,
    pub stem: String,
    pub skipped: Option<PathBuf>,
    pub published: Vec<PathBuf>,
    section_group: bool,
    artifacts: OutputArtifacts,
    created_directory: Option<PathBuf>,
    original_stem: String,
    index: u64,
    policy: ExistingFilePolicy,
    lock_root: PathBuf,
    reservation: Option<File>,
    _storage: storage_paths::StorageUseGuard,
}

impl OutputTransaction {
    pub fn new(
        destination: &Path,
        stem: &str,
        artifacts: OutputArtifacts,
        policy: ExistingFilePolicy,
        cancel: &AtomicBool,
    ) -> Result<Self, String> {
        Self::new_with_status(destination, stem, artifacts, policy, cancel, None)
    }
    pub fn new_with_status(
        destination: &Path,
        stem: &str,
        artifacts: OutputArtifacts,
        policy: ExistingFilePolicy,
        cancel: &AtomicBool,
        status: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
    ) -> Result<Self, String> {
        if artifacts
            .suffixes
            .iter()
            .any(|s| s.len() < 2 || s.contains(['/', '\\', ':']) || s.chars().any(char::is_control))
        {
            return Err("Invalid intended output artifact suffix.".into());
        }
        let storage = storage_paths::use_cache()?;
        storage_paths::validate_output_destination(destination)?;
        fs::create_dir_all(destination).map_err(|e| {
            format!(
                "Could not create output directory {}: {e}",
                destination.display()
            )
        })?;
        let destination = destination.canonicalize().map_err(|e| e.to_string())?;
        storage_paths::validate_output_destination(&destination)?;
        let lock_root = destination.join(".yt-dlp-gui-output");
        storage_paths::reject_links(&lock_root)?;
        let created = match fs::create_dir(&lock_root) {
            Ok(()) => true,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => false,
            Err(e) => return Err(format!("Could not coordinate output: {e}")),
        };
        let marker = lock_root.join("owner.txt");
        const OWNER: &str = "yt-dlp-gui-output/v1\n";
        if created {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&marker)
                .map_err(|e| e.to_string())?;
            file.write_all(OWNER.as_bytes())
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
        } else {
            // Another creator may still be writing the marker. Never adopt an
            // unknown directory, even if its name resembles our namespace.
            let mut valid = false;
            for _ in 0..20 {
                storage_paths::reject_links(&marker)?;
                if fs::read_to_string(&marker).is_ok_and(|text| text == OWNER) {
                    valid = true;
                    break;
                }
                if cancel.load(Ordering::Acquire) {
                    return Err("Download cancelled.".into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            if !valid {
                return Err(format!(
                    "Output coordination directory is not application-owned: {}",
                    lock_root.display()
                ));
            }
        }
        for entry in fs::read_dir(&lock_root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            storage_paths::reject_links(&path)?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let recognized = name == "owner.txt"
                || name.strip_suffix(".lock").is_some_and(|key| {
                    key.len() == 64
                        && key.bytes().all(|b| b.is_ascii_hexdigit())
                        && fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() == 0)
                });
            if !recognized {
                return Err(format!(
                    "Unrecognized output coordination file: {}",
                    path.display()
                ));
            }
        }
        let parent = storage_paths::cache_directory("yt-dlp-temp")?;
        let work = parent.join(unique_id()?);
        fs::create_dir(&work).map_err(|e| e.to_string())?;
        // Rust accepts long Windows paths internally; external Python/FFmpeg
        // consumers need the extended canonical spelling in their arguments.
        let work = work.canonicalize().map_err(|e| e.to_string())?;
        let original_stem = safe_stem(stem);
        let mut result = Self {
            created_directory: None,
            section_group: artifacts.sections,
            artifacts,
            status,
            #[cfg(test)]
            after_publish: None,
            plan: None,
            output_config: None,
            work,
            destination,
            stem: original_stem.clone(),
            original_stem,
            index: 0,
            policy,
            lock_root,
            reservation: None,
            skipped: None,
            published: Vec::new(),
            _storage: storage,
        };
        result.reserve(cancel)?;
        Ok(result)
    }

    fn collision(&self) -> Result<Option<PathBuf>, String> {
        for entry in fs::read_dir(&self.destination).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().to_lowercase();
            let stem = self.stem.to_lowercase();
            let matches_suffix = |suffix: &str| {
                self.artifacts
                    .suffixes
                    .iter()
                    .any(|s| s.eq_ignore_ascii_case(suffix))
            };
            let section = self.section_group
                && name
                    .strip_prefix(&format!("{stem} - "))
                    .and_then(|rest| rest.find('.').map(|dot| (&rest[..dot], &rest[dot..])))
                    .is_some_and(|(number, suffix)| {
                        number.len() >= 2
                            && number.bytes().all(|b| b.is_ascii_digit())
                            && matches_suffix(suffix)
                    });
            let file = !self.section_group && name.strip_prefix(&stem).is_some_and(matches_suffix);
            if (self.artifacts.directory && name == stem) || file || section {
                return Ok(Some(entry.path()));
            }
        }
        Ok(None)
    }

    pub fn include_sections(&mut self, cancel: &AtomicBool) -> Result<(), String> {
        self.section_group = true;
        if let Some(path) = self.collision()? {
            if self.policy == ExistingFilePolicy::Skip {
                self.skipped = Some(path);
            } else {
                self.reserve(cancel)?;
            }
        }
        Ok(())
    }

    fn reserve(&mut self, cancel: &AtomicBool) -> Result<(), String> {
        self.reservation = None;
        let mut reported_wait = false;
        loop {
            if cancel.load(Ordering::Acquire) {
                return Err("Download cancelled.".into());
            }
            self.index += 1;
            self.stem = if self.index == 1 {
                self.original_stem.clone()
            } else {
                format!("{} ({})", self.original_stem, self.index)
            };
            if self.index > 1_000_000 {
                return Err("Could not allocate an unused output filename.".into());
            }
            let key = super::sha256::sha256_hex_bytes(self.stem.to_lowercase().as_bytes());
            let path = self.lock_root.join(format!("{key}.lock"));
            storage_paths::reject_links(&path)?;
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&path)
                .map_err(|e| e.to_string())?;
            if file.metadata().map_err(|e| e.to_string())?.len() != 0 {
                return Err(format!(
                    "Unrecognized output coordination file: {}",
                    path.display()
                ));
            }
            if let Err(error) = file.try_lock() {
                if !matches!(error, std::fs::TryLockError::WouldBlock) {
                    return Err(format!("Output reservation failed: {error}"));
                }
                if self.policy == ExistingFilePolicy::KeepBoth {
                    continue;
                }
                if !reported_wait {
                    if let Some(status) = &self.status {
                        status("Output destination is in use. Waiting for it to become available.");
                    }
                    reported_wait = true;
                }
                self.index -= 1;
                std::thread::sleep(Duration::from_millis(30));
                continue;
            }
            self.reservation = Some(file);
            if let Some(path) = self.collision()? {
                if self.policy == ExistingFilePolicy::Skip {
                    self.skipped = Some(path);
                    return Ok(());
                }
                self.reservation = None;
                continue;
            }
            return Ok(());
        }
    }

    pub fn publish(
        &mut self,
        primary: &Path,
        cancel: &AtomicBool,
    ) -> Result<OutputOutcome, OutputFailure> {
        self.publish_files(primary, cancel)
            .map_err(|message| OutputFailure {
                disposition: if !self.published.is_empty() {
                    super::output_policy::OutputDisposition::PartialFailure
                } else if cancel.load(Ordering::Acquire) {
                    super::output_policy::OutputDisposition::Cancelled
                } else {
                    super::output_policy::OutputDisposition::Failed
                },
                published_paths: self.published.clone(),
                message,
            })
    }

    fn publish_files(
        &mut self,
        primary: &Path,
        cancel: &AtomicBool,
    ) -> Result<OutputOutcome, String> {
        if let Some(path) = &self.skipped {
            return Ok(OutputOutcome::SkippedExisting(path.clone()));
        }
        let primary = primary
            .canonicalize()
            .map_err(|e| format!("Completed output is missing: {e}"))?;
        let work = self.work.canonicalize().map_err(|e| e.to_string())?;
        if !primary.starts_with(&work) {
            return Err("Completed output escaped its private work directory.".into());
        }
        let mut files = Vec::new();
        if primary.is_dir() {
            collect_files(&primary, &mut files)?;
        } else {
            collect_files(&work, &mut files)?;
        }
        files.retain(|p| {
            let extension = p
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if primary.is_dir() {
                return !matches!(extension.as_str(), "part" | "ytdl" | "tmp" | "aria2");
            }
            let name = p.file_name().unwrap().to_string_lossy();
            let media = matches!(
                extension.as_str(),
                "mp4"
                    | "mkv"
                    | "webm"
                    | "mov"
                    | "flv"
                    | "avi"
                    | "mp3"
                    | "m4a"
                    | "flac"
                    | "wav"
                    | "opus"
                    | "aac"
                    | "ogg"
                    | "ts"
                    | "m2ts"
            );
            let stem = p.file_stem().unwrap().to_string_lossy();
            let complete_media_name = stem == "output"
                || stem.strip_prefix("output - ").is_some_and(|number| {
                    !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())
                });
            (primary.is_dir() || !media || complete_media_name)
                && !name.contains(".temp.")
                && !name.contains(".metadata-pass.")
                && !name.contains(".cover-pass.")
                && !matches!(
                    p.extension().and_then(|s| s.to_str()),
                    Some("part" | "ytdl" | "tmp" | "aria2")
                )
        });
        if !files.contains(&primary) && primary.is_file() {
            return Err("Completed output is not a regular owned artifact.".into());
        }
        if files.is_empty() {
            return Err("The tool produced no completed output artifacts.".into());
        }
        if !primary.is_dir()
            && files.iter().any(|p| {
                p.parent() != Some(work.as_path())
                    || !p
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .strip_prefix("output")
                        .is_some_and(|s| s.starts_with('.') || s.starts_with(" - "))
            })
        {
            return Err("Unexpected artifact outside the protected output group.".into());
        }
        files.sort_by_key(|p| if *p == primary { 0 } else { 1 });
        // Keep chapter directories as a single final group. Its files are
        // committed one by one into an exclusively created destination.
        if primary.is_dir() {
            loop {
                if cancel.load(Ordering::Acquire) {
                    return Err("Download cancelled.".into());
                }
                if let Some(path) = self.collision()? {
                    if self.policy == ExistingFilePolicy::Skip {
                        return Ok(OutputOutcome::SkippedExisting(path));
                    }
                    self.reserve(cancel)?;
                    continue;
                }
                let target = self.destination.join(&self.stem);
                match fs::create_dir(&target) {
                    Ok(()) => {
                        self.created_directory = Some(target.clone());
                        for source in &files {
                            let path = target
                                .join(source.strip_prefix(&primary).map_err(|e| e.to_string())?);
                            if cancel.load(Ordering::Acquire) {
                                return Err("Download cancelled.".into());
                            }
                            if let Some(parent) = path.parent() {
                                storage_paths::reject_links(parent)?;
                                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                            }
                            publish_file(source, &path, cancel)
                                .map_err(|e| format!("Output publication failed: {e}"))?;
                            self.published.push(path);
                        }
                        return Ok(OutputOutcome::Published(vec![target]));
                    }
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                        if self.policy == ExistingFilePolicy::Skip {
                            return Ok(OutputOutcome::SkippedExisting(target));
                        }
                        self.reserve(cancel)?;
                    }
                    Err(e) => return Err(format!("Output publication failed: {e}")),
                }
            }
        }
        loop {
            if cancel.load(Ordering::Acquire) {
                return Err("Download cancelled.".into());
            }
            storage_paths::validate_output_destination(&self.destination)?;
            if let Some(conflict) = self.collision()? {
                if self.policy == ExistingFilePolicy::Skip {
                    return Ok(OutputOutcome::SkippedExisting(conflict));
                }
                self.reserve(cancel)?;
                continue;
            }
            let planned: Vec<_> = files
                .iter()
                .map(|file| {
                    let relative = file.strip_prefix(&work).unwrap();
                    let name = relative.file_name().unwrap().to_string_lossy();
                    // Work filenames use a fixed literal stem so metadata cannot
                    // inject a final directory or a second naming template.
                    let mapped = name
                        .strip_prefix("output")
                        .map(|suffix| format!("{}{suffix}", self.stem))
                        .unwrap_or_else(|| name.to_string());
                    let parent = relative.parent().unwrap_or(Path::new(""));
                    self.destination.join(parent).join(mapped)
                })
                .collect();
            if let Some(conflict) = planned.iter().find(|p| p.exists()) {
                if self.policy == ExistingFilePolicy::Skip {
                    return Ok(OutputOutcome::SkippedExisting(conflict.clone()));
                }
                self.reserve(cancel)?;
                continue;
            }
            for (source, target) in files.iter().zip(&planned) {
                if cancel.load(Ordering::Acquire) {
                    return Err("Download cancelled.".into());
                }
                storage_paths::validate_output_destination(target)?;
                let parent = target.parent().ok_or("Output has no parent")?;
                storage_paths::reject_links(parent)?;
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                match publish_file(source, target, cancel) {
                    Ok(()) => {
                        self.published.push(target.clone());
                        #[cfg(test)]
                        if let Some(hook) = &mut self.after_publish {
                            hook(target);
                        }
                    }
                    Err(error)
                        if error.kind() == io::ErrorKind::AlreadyExists
                            && self.published.is_empty() =>
                    {
                        if self.policy == ExistingFilePolicy::Skip {
                            return Ok(OutputOutcome::SkippedExisting(target.clone()));
                        }
                        self.reserve(cancel)?;
                        break;
                    }
                    Err(error) => {
                        return Err(format!("Output publication failed: {error}"));
                    }
                }
            }
            if self.published.len() == files.len() {
                return Ok(OutputOutcome::Published(self.published.clone()));
            }
        }
    }
}

impl Drop for OutputTransaction {
    fn drop(&mut self) {
        let _ = storage_paths::remove_owned_tree(&self.work);
        if self.published.is_empty() {
            if let Some(path) = &self.created_directory {
                let _ = fs::remove_dir(path);
            }
        }
    }
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    storage_paths::reject_links(root)?;
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path
            .file_name()
            .is_some_and(|s| s == ".incoming" || s == ".conversion")
        {
            continue;
        }
        storage_paths::reject_links(&path)?;
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if metadata.is_dir() {
            collect_files(&path, files)?;
        } else if metadata.is_file() {
            files.push(path);
        } else {
            return Err(format!("Unexpected output artifact: {}", path.display()));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn move_new(source: &Path, target: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(source: *const u16, target: *const u16, flags: u32) -> i32;
    }
    let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<_> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), 0x8) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
#[cfg(not(windows))]
fn move_new(source: &Path, target: &Path) -> io::Result<()> {
    fs::hard_link(source, target)?;
    let _ = fs::remove_file(source);
    Ok(())
}

pub(crate) fn publish_file(source: &Path, target: &Path, cancel: &AtomicBool) -> io::Result<()> {
    check_cancel(cancel)?;
    match move_new(source, target) {
        Ok(()) => return Ok(()),
        Err(e) if cross_device(&e) => {}
        Err(e) => return Err(e),
    }
    publish_copy(source, target, cancel)
}

fn cross_device(error: &io::Error) -> bool {
    error.raw_os_error() == Some(if cfg!(windows) { 17 } else { 18 })
}

fn check_cancel(cancel: &AtomicBool) -> io::Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "Download cancelled.",
        ))
    } else {
        Ok(())
    }
}

fn publish_copy(source: &Path, target: &Path, cancel: &AtomicBool) -> io::Result<()> {
    let temp = target.with_file_name(format!(
        ".yt-dlp-gui-publish-{}.tmp",
        unique_id().map_err(io::Error::other)?
    ));
    let mut owns_temp = false;
    let result = (|| {
        check_cancel(cancel)?;
        let mut input = File::open(source)?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)?;
        owns_temp = true;
        let mut buffer = vec![0u8; 1024 * 1024];
        loop {
            check_cancel(cancel)?;
            let count = input.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            output.write_all(&buffer[..count])?;
        }
        let metadata = input.metadata()?;
        output.set_times(
            fs::FileTimes::new()
                .set_modified(metadata.modified()?)
                .set_accessed(metadata.accessed()?),
        )?;
        output.sync_all()?;
        drop(output);
        drop(input);
        if hash_file(source)? != hash_file(&temp)? {
            return Err(io::Error::other(
                "Output publication copy failed verification",
            ));
        }
        check_cancel(cancel)?;
        move_new(&temp, target)?;
        // The final file is already committed. A cleanup failure cannot turn
        // that established path into an unreported/unpublished artifact.
        if let Err(error) = fs::remove_file(source) {
            eprintln!(
                "[output] Published {}; source cleanup retained: {error}",
                target.display()
            );
        }
        Ok(())
    })();
    if owns_temp {
        let _ = fs::remove_file(temp);
    }
    result
}

#[cfg(test)]
#[path = "output_transaction_tests.rs"]
mod tests;

fn hash_file(path: &Path) -> io::Result<String> {
    super::sha256::sha256_hex_file(path).map_err(io::Error::other)
}
