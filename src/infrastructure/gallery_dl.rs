use std::{
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::{Map, Value};

use crate::domain::{
    GalleryAnalysis, GalleryDownloadArtifact, GalleryMediaItem, GalleryPost, GalleryQueuedUrl,
};

use super::{
    configure_background_command, resolve_tool_path, run_tracked_command_output,
    tools::{ResolvedCookieSource, ToolPaths},
};

pub const DEFAULT_GALLERY_DL_PATH: &str = ".\\tools\\gallery-dl\\gallery-dl.exe";
pub const GALLERY_POST_COMPLETED_MARKER: &str = "__YTDLPGUI_GALLERY_POST_COMPLETED__";
const MESSAGE_ERROR: i64 = -1;
const MESSAGE_DIRECTORY: i64 = 2;
const MESSAGE_URL: i64 = 3;
const MESSAGE_QUEUE: i64 = 6;
#[path = "gallery_analysis_policy.rs"]
mod analysis_policy;
#[path = "gallery_metadata_diagnostics.rs"]
mod metadata_diagnostics;

#[derive(Clone, Debug)]
pub struct GalleryDlRuntime {
    program: PathBuf,
    #[cfg(test)]
    source_harness: bool,
}

impl GalleryDlRuntime {
    pub fn standalone(path: impl Into<PathBuf>) -> Self {
        Self {
            program: path.into(),
            #[cfg(test)]
            source_harness: false,
        }
    }

    fn availability_error(&self) -> Option<String> {
        if !self.program.is_file() {
            return Some(format!(
                "gallery-dl executable was not found: {}.",
                self.program.display()
            ));
        }
        self.acquire().err()
    }

    #[cfg(test)]
    pub(crate) fn source_harness(path: impl Into<PathBuf>) -> Self {
        Self {
            program: path.into(),
            source_harness: true,
        }
    }

    fn acquire(
        &self,
    ) -> Result<Option<super::gallery_runtime_policy::GalleryRuntimeGuard>, String> {
        #[cfg(test)]
        if self.source_harness {
            return Ok(None);
        }
        super::gallery_runtime_policy::acquire(&self.program).map(Some)
    }

    fn command(&self) -> Command {
        Command::new(&self.program)
    }
}

impl Default for GalleryDlRuntime {
    fn default() -> Self {
        Self::standalone(resolve_tool_path(DEFAULT_GALLERY_DL_PATH))
    }
}

#[derive(Clone, Debug, Default)]
pub struct GalleryDlBackend {
    runtime: GalleryDlRuntime,
}

impl GalleryDlBackend {
    pub fn new(runtime: GalleryDlRuntime) -> Self {
        Self { runtime }
    }

    pub fn from_tool_paths(tool_paths: &ToolPaths) -> Self {
        Self::new(GalleryDlRuntime::standalone(resolve_tool_path(
            &tool_paths.gallery_dl,
        )))
    }

    pub fn executable_path(&self) -> &Path {
        &self.runtime.program
    }

    pub fn is_available(&self) -> bool {
        self.runtime.availability_error().is_none()
    }

    pub fn analyze_url(
        &self,
        url: &str,
        tool_paths: &ToolPaths,
        use_cookies: bool,
    ) -> Result<GalleryDlAnalyzeOutput, String> {
        self.analyze_url_detailed(url, tool_paths, use_cookies)
            .map_err(|error| error.message)
    }

    pub(crate) fn analyze_url_detailed(
        &self,
        url: &str,
        tool_paths: &ToolPaths,
        use_cookies: bool,
    ) -> Result<GalleryDlAnalyzeOutput, super::AnalyzeError> {
        self.analyze_metadata_detailed(url, tool_paths, use_cookies, true)
    }

    /// A proven root Dispatch only needs its Queue messages to count feeds.
    /// Resolving the children here would re-fetch posts before every album read
    /// (and erase the very Queue boundaries this guard is meant to inspect).
    /// Unproved/non-dispatch sources keep their existing resolved probe.
    pub(crate) fn supports_bounded_timeline_feed(
        &self,
        url: &str,
        tool_paths: &ToolPaths,
        use_cookies: bool,
    ) -> Result<bool, String> {
        let (_, metadata) = super::extractor_index::matching_extractor_metadata(url)?;
        let dispatch_only = analysis_policy::is_proven_root_dispatch(metadata.as_ref());
        let analysis = self
            .analyze_metadata_detailed(url, tool_paths, use_cookies, !dispatch_only)
            .map_err(|error| error.message)?
            .analysis;
        let supported = analysis.supports_single_feed_post_range();
        super::operation_diagnostics::record(
            "gallery.feed.scope",
            serde_json::json!({
                "dispatchOnly": dispatch_only,
                "queuedCount": analysis.queued_urls.len(),
                "postCount": analysis.posts.len(), "mediaCount": analysis.media.len(),
                "singleFeed": supported,
            }),
        );
        Ok(supported)
    }

    fn analyze_metadata_detailed(
        &self,
        url: &str,
        tool_paths: &ToolPaths,
        use_cookies: bool,
        resolve_intermediaries: bool,
    ) -> Result<GalleryDlAnalyzeOutput, super::AnalyzeError> {
        let _storage_lease =
            super::storage_paths::use_cache().map_err(|message| super::AnalyzeError {
                message,
                command_line: None,
            })?;
        let (mut command, _runtime_guard) =
            self.base_command().map_err(|message| super::AnalyzeError {
                message,
                command_line: None,
            })?;
        command
            .arg("--config-ignore")
            .arg("--no-input")
            .arg("--no-colors")
            .arg(if resolve_intermediaries {
                "-J"
            } else {
                "--dump-json"
            });
        let _storage_work = super::storage_paths::CacheWorkDirectory::new().map_err(|message| {
            super::AnalyzeError {
                message,
                command_line: None,
            }
        })?;
        append_gallery_storage_args(&mut command, Some(&_storage_work.path)).map_err(
            |message| super::AnalyzeError {
                message,
                command_line: None,
            },
        )?;
        append_gallery_ytdl_ffmpeg_args(&mut command, tool_paths);
        if !resolve_intermediaries {
            command.arg("-o").arg("cookies-update=false");
        }
        // Two posts are a routing/scope probe, not the card's media total.
        analysis_policy::append_post_window(
            &mut command,
            1,
            analysis_policy::ANALYSIS_POST_LIMIT,
            false,
            None,
        );
        append_shared_cookie_args(&mut command, tool_paths, use_cookies, url).map_err(
            |message| super::AnalyzeError {
                message,
                command_line: None,
            },
        )?;
        // The exact URL supplied by the extension is also supplied to gallery-dl.
        command
            .arg(url)
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONUNBUFFERED", "1");

        let command_line = format_command_line(&command);
        let failure = |message: String| super::AnalyzeError {
            message,
            command_line: Some(command_line.clone()),
        };
        let stage = if resolve_intermediaries {
            "gallery-dl analyze"
        } else {
            "gallery-dl root dispatch"
        };
        let output = run_tracked_command_output(&mut command, stage)
            .map_err(|error| failure(format!("Could not start gallery-dl: {error}")))?;
        if !output.status.success() {
            metadata_diagnostics::record_metadata_result(resolve_intermediaries, &output, None);
            return Err(failure(process_failure_message(
                &output.stderr,
                &output.stdout,
            )));
        }

        let parsed = parse_gallery_dl_json(url, &output.stdout);
        metadata_diagnostics::record_metadata_result(
            resolve_intermediaries,
            &output,
            Some(&parsed),
        );
        let mut analysis = parsed.map_err(|message| {
            let detail = String::from_utf8_lossy(&output.stderr);
            let detail = detail.trim();
            failure(if detail.is_empty() {
                message
            } else {
                format!("{message}\n\n{detail}")
            })
        })?;
        // Auxiliary scope lookup must not invalidate successful forced analysis.
        analysis.root_extractor = super::extractor_index::matching_gallery_extractor(url)
            .ok()
            .flatten()
            .unwrap_or_default();
        Ok(GalleryDlAnalyzeOutput {
            analysis,
            command_line,
        })
    }

    /// Browser-owned bytes need gallery-dl's actual archive key, which DataJob
    /// (-J) does not compute. DownloadJob with download=false emits after/skip
    /// metadata without downloading, persisting an archive or updating cookies.
    /// Desktop's bounded -J analysis remains independent and unchanged.
    pub fn analyze_browser_url(
        &self,
        url: &str,
        tool_paths: &ToolPaths,
        use_cookies: bool,
    ) -> Result<GalleryDlAnalyzeOutput, String> {
        self.analyze_browser_window(
            url,
            tool_paths,
            use_cookies,
            None,
            1,
            analysis_policy::ANALYSIS_POST_LIMIT,
        )
    }

    pub fn analyze_browser_window(
        &self,
        url: &str,
        tool_paths: &ToolPaths,
        use_cookies: bool,
        range: Option<&crate::domain::GalleryDownloadRange>,
        start: u32,
        end: u32,
    ) -> Result<GalleryDlAnalyzeOutput, String> {
        self.analyze_browser_window_stream(
            url,
            tool_paths,
            use_cookies,
            range,
            start,
            end,
            |_, _| Ok(()),
        )
    }

    pub fn analyze_browser_window_stream(
        &self,
        url: &str,
        tool_paths: &ToolPaths,
        use_cookies: bool,
        range: Option<&crate::domain::GalleryDownloadRange>,
        start: u32,
        end: u32,
        on_post: impl FnMut(&GalleryAnalysis, usize) -> Result<(), String>,
    ) -> Result<GalleryDlAnalyzeOutput, String> {
        let destination = super::storage_paths::require_ready()?
            .cache_root
            .join("gallery-dl-temp/metadata");
        let mut prepared = self.prepare_item_command(url, &destination, tool_paths, use_cookies)?;
        prepared
            .command
            .arg("-o")
            .arg("download=false")
            .arg("-o")
            .arg("directory-create=false")
            .arg("-o")
            .arg("output.mode=false")
            .arg("-o")
            .arg("cookies-update=false");
        // Browser playback must retain the same alternate URLs that the native
        // downloader tries. Do not expose headers or other private runtime data.
        prepared.command.arg("-o").arg(format!(
            "postprocessors={}",
            gallery_metadata_postprocessors(true)
        ));
        let filter = range
            .map(crate::domain::GalleryDownloadRange::post_filter)
            .transpose()?
            .flatten();
        let root_metadata = super::extractor_index::matching_extractor_metadata(url)
            .ok()
            .and_then(|(_, gallery)| gallery);
        analysis_policy::append_post_window(
            &mut prepared.command,
            start,
            end,
            filter.is_some(),
            root_metadata.as_ref().map(|metadata| metadata.key.as_str()),
        );
        if let Some(filter) = filter {
            prepared.command.arg("--post-filter").arg(filter);
        }
        let command_line = format_command_line(&prepared.command);
        let mut analysis =
            super::gallery_browser_stream::read_posts(prepared, url, range.is_some(), on_post)?;
        if analysis.media.is_empty() && range.is_none() {
            return Err("gallery-dl did not emit browser media metadata.".into());
        }
        analysis.root_extractor = super::extractor_index::matching_gallery_extractor(url)
            .ok()
            .flatten()
            .unwrap_or_default();
        Ok(GalleryDlAnalyzeOutput {
            analysis,
            command_line,
        })
    }

    /// Prepare a desktop or browser-native Gallery item download. No persistent
    /// gallery-dl archive or metadata sidecar: gallery-dl computes its native
    /// archive key in memory and emits completed/skipped media metadata as
    /// JSONL for v2's XMP worker. `after` and `skip` are mutually exclusive in
    /// gallery-dl, so every downloaded or already-existing artifact is handled
    /// exactly once.
    pub fn prepare_item_download(
        &self,
        url: &str,
        destination: &Path,
        tool_paths: &ToolPaths,
        use_cookies: bool,
    ) -> Result<PreparedGalleryDlDownload, String> {
        super::storage_paths::validate_output_destination(destination)?;
        self.prepare_item_command(url, destination, tool_paths, use_cookies)
    }

    // Metadata-only callers use an owned nominal path and disable all media
    // writes before execution. Final-download callers validate ownership above.
    fn prepare_item_command(
        &self,
        url: &str,
        destination: &Path,
        tool_paths: &ToolPaths,
        use_cookies: bool,
    ) -> Result<PreparedGalleryDlDownload, String> {
        if destination.as_os_str().is_empty() {
            return Err("gallery-dl download destination is empty.".to_owned());
        }

        let (mut command, runtime_guard) = self.base_command()?;
        let storage_work = super::storage_paths::CacheWorkDirectory::new()?;
        command
            .arg("--config-ignore")
            .arg("--no-input")
            .arg("--no-colors")
            .arg("--windows-filenames")
            .arg("--destination")
            .arg(destination)
            // Keep gallery-dl's own archive-key calculation without leaving a
            // persistent SQLite file behind. `_archive_key` is emitted by the
            // metadata postprocessor and persisted losslessly by v2 in XMP.
            .arg("--download-archive")
            .arg(":memory:")
            // -O is global to every postprocessor, so separate typed options
            // are required: otherwise it turns post markers into duplicate JSON.
            // ASCII JSON protects Unicode from Windows pipe code-page replacement;
            // private fields retain the exact native archive key and artifact path.
            .arg("-o")
            .arg(format!(
                "postprocessors={}",
                gallery_metadata_postprocessors(false)
            ));
        append_shared_cookie_args(&mut command, tool_paths, use_cookies, url)?;
        append_gallery_storage_args(&mut command, Some(&storage_work.path))?;
        append_gallery_ytdl_ffmpeg_args(&mut command, tool_paths);
        // MetadataPP writes stdout without flushing; stream events as they happen.
        command
            .arg(url)
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONUNBUFFERED", "1");

        let command_line = format_command_line(&command);
        Ok(PreparedGalleryDlDownload {
            command,
            destination: destination.to_path_buf(),
            command_line,
            storage_work,
            runtime_guard,
        })
    }

    fn base_command(
        &self,
    ) -> Result<
        (
            Command,
            Option<super::gallery_runtime_policy::GalleryRuntimeGuard>,
        ),
        String,
    > {
        let runtime_guard = self.runtime.acquire()?;
        let mut command = self.runtime.command();
        configure_background_command(&mut command);
        analysis_policy::append_profile_feed_policy(&mut command);
        Ok((command, runtime_guard))
    }
}

pub struct PreparedGalleryDlDownload {
    pub command: Command,
    pub destination: PathBuf,
    pub command_line: String,
    pub(crate) storage_work: super::storage_paths::CacheWorkDirectory,
    pub(crate) runtime_guard: Option<super::gallery_runtime_policy::GalleryRuntimeGuard>,
}

fn gallery_metadata_postprocessors(browser_sources: bool) -> serde_json::Value {
    // The ytdl extractor carries a self-referencing info dictionary and a live
    // YoutubeDL object. Neither is metadata transport or safe to JSON-encode.
    let mut excluded = vec![
        "_ytdl_manifest_data",
        "_ytdl_info_dict",
        "_ytdl_instance",
        "_http_headers",
    ];
    if !browser_sources {
        excluded.push("_fallback");
    }
    serde_json::json!([
        {
            "name": "metadata", "event": ["after", "skip"],
            "mode": "jsonl", "ascii": true, "filename": "-", "private": true,
            "exclude": excluded
        },
        {
            "name": "metadata", "event": "post-after", "mode": "print",
            "content-format": GALLERY_POST_COMPLETED_MARKER
        }
    ])
}

/// Time filters skip old pins; only the recognized Instagram posts feed may
/// stop at an old unpinned post. Unknown ordering retains a full scan.
/// Missing or incompatible `date` fails closed.
pub fn apply_timeline_download_range(
    prepared: &mut PreparedGalleryDlDownload,
    range: &crate::domain::GalleryDownloadRange,
) -> Result<(), String> {
    let filter = range.post_filter()?;
    if let crate::domain::GalleryDownloadRange::Posts { count } = range {
        prepared
            .command
            .arg("--post-range")
            .arg(format!("1-{count}"));
    }
    if let Some(filter) = filter {
        prepared.command.arg("--post-filter").arg(filter);
    }
    prepared.command_line = format_command_line(&prepared.command);
    Ok(())
}

pub struct GalleryDlAnalyzeOutput {
    pub analysis: GalleryAnalysis,
    pub command_line: String,
}

#[derive(Clone, Debug)]
pub struct GalleryDlMetadataEvent {
    pub artifact: GalleryDownloadArtifact,
    /// Private runtime fields are removed after extracting the artifact and
    /// bounded browser candidates; canonical mapping never sees these fields.
    pub metadata: Value,
    /// Bounded runtime-only transport, removed before canonical metadata mapping.
    pub browser_fallback_urls: Vec<String>,
}

pub fn parse_gallery_dl_download_event(
    line: &str,
) -> Option<Result<GalleryDlMetadataEvent, String>> {
    let payload = line.trim();
    if !payload.starts_with('{') {
        return None;
    }

    let mut metadata = match serde_json::from_str::<Value>(payload) {
        Ok(value) => value,
        Err(error) => match escape_backslashes_in_json_strings_with_invalid_escape(payload)
            .and_then(|repaired| serde_json::from_str::<Value>(&repaired).ok())
        {
            Some(value) => value,
            None => {
                return Some(Err(format!(
                    "Could not parse gallery-dl metadata event: {error}"
                )));
            }
        },
    };
    let Some(object) = metadata.as_object_mut() else {
        return None;
    };
    let path = object.get("_path").and_then(value_text).unwrap_or_default();
    let archive_key = object
        .get("_archive_key")
        .and_then(value_text)
        .unwrap_or_default();
    if path.is_empty() || archive_key.is_empty() {
        return None;
    }

    let artifact = GalleryDownloadArtifact {
        date: None,
        path: PathBuf::from(path),
        archive_key,
        media_url: object.get("_url").and_then(value_text).unwrap_or_default(),
        category: object
            .get("category")
            .and_then(value_text)
            .unwrap_or_default(),
        subcategory: object
            .get("subcategory")
            .and_then(value_text)
            .unwrap_or_default(),
        sequence: object.get("num").and_then(value_u64),
        media_type: object.get("type").and_then(value_text).unwrap_or_default(),
        filename: object
            .get("filename")
            .and_then(value_text)
            .unwrap_or_default(),
        extension: object
            .get("extension")
            .and_then(value_text)
            .unwrap_or_default(),
    };

    let browser_fallback_urls = object
        .get("_fallback")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|url| url.len() <= 8192)
        .take(16)
        .map(str::to_owned)
        .collect();
    object.retain(|key, _| !key.starts_with('_'));
    Some(Ok(GalleryDlMetadataEvent {
        artifact,
        metadata,
        browser_fallback_urls,
    }))
}

/// Repair gallery-dl runtimes that place literal backslashes in JSON string
/// values. This runs only after strict parsing fails and changes only complete
/// strings that contain at least one invalid JSON escape.
fn escape_backslashes_in_json_strings_with_invalid_escape(payload: &str) -> Option<String> {
    let bytes = payload.as_bytes();
    let mut invalid_string_ranges = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }

        let value_start = index + 1;
        let mut has_invalid_escape = false;
        index = value_start;
        while index < bytes.len() {
            match bytes[index] {
                b'"' => {
                    if has_invalid_escape {
                        invalid_string_ranges.push((value_start, index));
                    }
                    index += 1;
                    break;
                }
                b'\\' => {
                    let Some(escaped) = bytes.get(index + 1).copied() else {
                        has_invalid_escape = true;
                        index += 1;
                        continue;
                    };
                    let valid_escape = match escaped {
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => true,
                        b'u' => bytes
                            .get(index + 2..index + 6)
                            .is_some_and(|digits| digits.iter().all(u8::is_ascii_hexdigit)),
                        _ => false,
                    };
                    has_invalid_escape |= !valid_escape;
                    index += if escaped == b'u' && valid_escape {
                        6
                    } else {
                        2
                    };
                }
                _ => index += 1,
            }
        }
    }

    if invalid_string_ranges.is_empty() {
        return None;
    }

    let added_capacity = invalid_string_ranges
        .iter()
        .map(|(start, end)| {
            payload[*start..*end]
                .bytes()
                .filter(|byte| *byte == b'\\')
                .count()
        })
        .sum::<usize>();
    let mut repaired = String::with_capacity(payload.len() + added_capacity);
    let mut copied_until = 0;
    for (start, end) in invalid_string_ranges {
        repaired.push_str(&payload[copied_until..start]);
        for ch in payload[start..end].chars() {
            if ch == '\\' {
                repaired.push('\\');
            }
            repaired.push(ch);
        }
        copied_until = end;
    }
    repaired.push_str(&payload[copied_until..]);
    Some(repaired)
}

fn value_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.trim().parse::<u64>().ok())
}

pub fn parse_gallery_dl_json(source_url: &str, bytes: &[u8]) -> Result<GalleryAnalysis, String> {
    let messages = serde_json::from_slice::<Value>(bytes)
        .map_err(|error| format!("Could not parse gallery-dl JSON: {error}"))?;
    let messages = messages
        .as_array()
        .ok_or_else(|| "gallery-dl JSON output is not an event list.".to_owned())?;

    let mut analysis = GalleryAnalysis {
        source_url: source_url.to_owned(),
        ..GalleryAnalysis::default()
    };
    let mut current_post_index = None;

    for message in messages {
        let Some(parts) = message.as_array() else {
            continue;
        };
        let Some(kind) = parts.first().and_then(Value::as_i64) else {
            continue;
        };
        match kind {
            MESSAGE_ERROR => return Err(gallery_dl_json_error(parts)),
            MESSAGE_DIRECTORY => {
                let metadata = parts.get(1).and_then(Value::as_object);
                analysis.posts.push(build_gallery_post(metadata));
                current_post_index = Some(analysis.posts.len() - 1);
            }
            MESSAGE_URL => {
                let Some(url) = parts.get(1).and_then(Value::as_str) else {
                    continue;
                };
                let metadata = parts.get(2).and_then(Value::as_object);
                analysis.media.push(build_gallery_media_item(
                    current_post_index,
                    source_url,
                    url,
                    metadata,
                ));
            }
            MESSAGE_QUEUE => {
                let Some(url) = parts.get(1).and_then(Value::as_str) else {
                    continue;
                };
                let metadata = parts.get(2).and_then(Value::as_object);
                analysis.queued_urls.push(GalleryQueuedUrl {
                    url: url.to_owned(),
                    category: text_field(metadata, &["category"]),
                    subcategory: text_field(metadata, &["subcategory"]),
                });
            }
            _ => {}
        }
    }

    if analysis.posts.is_empty() && analysis.media.is_empty() && analysis.queued_urls.is_empty() {
        return Err("gallery-dl did not find downloadable media.".to_owned());
    }
    Ok(analysis)
}

pub(super) fn build_gallery_post(metadata: Option<&Map<String, Value>>) -> GalleryPost {
    GalleryPost {
        id: text_field(metadata, &["post_id", "tweet_id", "id", "gallery_id"]),
        title: text_field(
            metadata,
            &[
                "title",
                "post_title",
                "gallery_title",
                "content",
                "description",
            ],
        ),
        author: text_field(metadata, &["author", "username", "user", "account"]),
        author_nick: metadata
            .and_then(|meta| meta.get("author"))
            .and_then(Value::as_object)
            .map(|author| text_field(Some(author), &["nick"]))
            .unwrap_or_default(),
        author_name: metadata
            .and_then(|meta| meta.get("author"))
            .and_then(Value::as_object)
            .map(|author| text_field(Some(author), &["name"]))
            .unwrap_or_default(),
        author_id: metadata
            .and_then(|meta| meta.get("author"))
            .and_then(Value::as_object)
            .map(|author| text_field(Some(author), &["id"]))
            .unwrap_or_default(),
        category: text_field(metadata, &["category"]),
        subcategory: text_field(metadata, &["subcategory"]),
        date: text_field(metadata, &["date"]),
        content: text_field(metadata, &["content", "description"]),
        thumbnail_url: {
            let thumbnail = text_field(metadata, &["thumbnail", "preview", "cover"]);
            if thumbnail.is_empty() {
                metadata
                    .and_then(|meta| meta.get("author"))
                    .and_then(|author| author.get("profile_image"))
                    .and_then(value_text)
                    .unwrap_or_default()
            } else {
                thumbnail
            }
        },
        source_metadata: metadata.cloned().map(Value::Object).unwrap_or(Value::Null),
    }
}

pub(super) fn build_gallery_media_item(
    post_index: Option<usize>,
    source_page_url: &str,
    url: &str,
    metadata: Option<&Map<String, Value>>,
) -> GalleryMediaItem {
    let filename = text_field(metadata, &["filename", "name"]);
    let extracted_source_url = text_field(metadata, &["_url", "source_url", "post_url"]);
    GalleryMediaItem {
        post_index,
        url: url.to_owned(),
        source_page_url: if extracted_source_url.is_empty() {
            source_page_url.to_owned()
        } else {
            extracted_source_url
        },
        post_id: text_field(metadata, &["post_id", "tweet_id", "gallery_id"]),
        media_id: {
            let id = text_field(metadata, &["media_id", "photo_id", "image_id", "id"]);
            if id.is_empty() { filename.clone() } else { id }
        },
        sequence: unsigned_field(metadata, &["num", "index", "page"]),
        media_type: text_field(metadata, &["type", "media_type"]),
        filename,
        extension: text_field(metadata, &["extension", "ext"]),
        width: unsigned_field(metadata, &["width", "image_width"]),
        height: unsigned_field(metadata, &["height", "image_height"]),
        archive_key: String::new(),
        output_path: None,
        browser_relative_path: None,
        browser_fallback_urls: Vec::new(),
        source_metadata: metadata.cloned().map(Value::Object).unwrap_or(Value::Null),
    }
}

fn text_field(metadata: Option<&Map<String, Value>>, keys: &[&str]) -> String {
    let Some(metadata) = metadata else {
        return String::new();
    };
    keys.iter()
        .find_map(|key| metadata.get(*key).and_then(value_text))
        .unwrap_or_default()
}

fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_owned())
        }
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn unsigned_field(metadata: Option<&Map<String, Value>>, keys: &[&str]) -> Option<u64> {
    let metadata = metadata?;
    keys.iter().find_map(|key| {
        let value = metadata.get(*key)?;
        value
            .as_u64()
            .or_else(|| value.as_str()?.trim().parse::<u64>().ok())
    })
}

fn gallery_dl_json_error(parts: &[Value]) -> String {
    let metadata = parts.get(1).and_then(Value::as_object);
    let error = text_field(metadata, &["error"]);
    let message = text_field(metadata, &["message"]);
    match (error.is_empty(), message.is_empty()) {
        (false, false) => format!("gallery-dl analysis failed: {error}: {message}"),
        (false, true) => format!("gallery-dl analysis failed: {error}"),
        (true, false) => format!("gallery-dl analysis failed: {message}"),
        (true, true) => "gallery-dl analysis failed.".to_owned(),
    }
}

fn append_shared_cookie_args(
    command: &mut Command,
    tool_paths: &ToolPaths,
    use_cookies: bool,
    url: &str,
) -> Result<(), String> {
    match tool_paths.resolve_shared_cookie_source(use_cookies, Some(url))? {
        Some(ResolvedCookieSource::File(path)) => {
            command.arg("--cookies").arg(path);
        }
        Some(ResolvedCookieSource::Browser(browser)) => {
            command.arg("--cookies-from-browser").arg(browser);
        }
        None => {}
    }
    Ok(())
}

fn append_gallery_storage_args(command: &mut Command, work: Option<&Path>) -> Result<(), String> {
    let cachedir = super::storage_paths::cache_directory("yt-dlp")?;
    let raw_options = serde_json::json!({"cachedir": cachedir});
    // Extractor-level settings also cover ytdl-generic and prepared yt-dlp instances.
    for owner in ["extractor", "downloader.ytdl"] {
        command
            .arg("-o")
            .arg(format!("{owner}.raw-options={raw_options}"));
    }
    if let Some(work) = work {
        let runtime_temp = work.join("runtime-temp");
        super::storage_paths::reject_links(&runtime_temp)?;
        std::fs::create_dir_all(&runtime_temp)
            .map_err(|e| format!("Could not create gallery runtime temporary folder: {e}"))?;
        command
            .env("TEMP", &runtime_temp)
            .env("TMP", &runtime_temp)
            .env("PYTHONDONTWRITEBYTECODE", "1");
        command
            .arg("-o")
            .arg("part=true")
            .arg("-o")
            .arg(format!("part-directory={}", serde_json::json!(work)));
    }
    Ok(())
}

fn append_gallery_ytdl_ffmpeg_args(command: &mut Command, tool_paths: &ToolPaths) {
    let ffmpeg_path = resolve_tool_path(&tool_paths.ffmpeg);
    if !ffmpeg_path.is_file() {
        return;
    }
    let path = ffmpeg_path.to_string_lossy().replace('\\', "/");
    let path = path.replace('"', "\\\"");
    command.arg("-o").arg(format!(
        "downloader.ytdl.cmdline-args=--ffmpeg-location \"{path}\""
    ));
}

fn process_failure_message(stderr: &[u8], stdout: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(stdout).trim().to_owned();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "gallery-dl exited without an error message".to_owned()
    };
    format!("gallery-dl analysis failed: {detail}")
}

fn format_command_line(command: &Command) -> String {
    std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|part| format!("\"{}\"", part.to_string_lossy().replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(" ")
}
