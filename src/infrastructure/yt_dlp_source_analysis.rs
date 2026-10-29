//! Shared desktop/Native Host routing before full playlist format extraction.
//! Static evidence is optional; unknown sources use a two-entry flat probe.
use super::extractor_index::{YtSourceScope, matching_yt_source_scope};
use super::{AnalyzeError, AnalyzeOutput, ToolPaths};
use serde_json::Value;
use std::sync::atomic::AtomicBool;

pub(crate) enum DesktopSourceAnalysis {
    Playlist,
    Single(AnalyzeOutput),
}

/// Source-only callers do not need to resolve formats for a known single URL.
/// Unknown single probes carry reusable metadata for compact audio insertion.
pub(crate) enum DesktopSourcePreflight {
    Playlist,
    Single(Option<AnalyzeOutput>),
}

pub(crate) fn preflight_desktop_source(
    paths: &ToolPaths,
    url: &str,
    use_cookies: bool,
) -> Result<DesktopSourcePreflight, AnalyzeError> {
    match matching_yt_source_scope(url).map_err(|message| AnalyzeError {
        message,
        command_line: None,
    })? {
        YtSourceScope::Playlist => Ok(DesktopSourcePreflight::Playlist),
        YtSourceScope::Single => Ok(DesktopSourcePreflight::Single(None)),
        YtSourceScope::Any | YtSourceScope::Unknown => {
            probe_unknown_source(paths, url, use_cookies).map(|output| match output {
                DesktopSourceAnalysis::Playlist => DesktopSourcePreflight::Playlist,
                DesktopSourceAnalysis::Single(output) => {
                    DesktopSourcePreflight::Single(Some(output))
                }
            })
        }
    }
}

pub(crate) fn analyze_desktop_source(
    paths: &ToolPaths,
    url: &str,
    use_cookies: bool,
) -> Result<DesktopSourceAnalysis, AnalyzeError> {
    analyze_desktop_source_cancellable(paths, url, use_cookies, &AtomicBool::new(false))
}

pub(crate) fn analyze_desktop_source_cancellable(
    paths: &ToolPaths,
    url: &str,
    use_cookies: bool,
    cancel: &AtomicBool,
) -> Result<DesktopSourceAnalysis, AnalyzeError> {
    match matching_yt_source_scope(url).map_err(|message| AnalyzeError {
        message,
        command_line: None,
    })? {
        YtSourceScope::Playlist => return Ok(DesktopSourceAnalysis::Playlist),
        YtSourceScope::Single => {
            let tool = paths
                .validate_yt_dlp_available()
                .map_err(|message| AnalyzeError {
                    message,
                    command_line: None,
                })?;
            return paths
                .run_analyze_command_cancellable(&tool, url, use_cookies, None, None, None, cancel)
                .map(DesktopSourceAnalysis::Single);
        }
        YtSourceScope::Any | YtSourceScope::Unknown => {}
    }
    probe_unknown_source_cancellable(paths, url, use_cookies, cancel)
}

fn probe_unknown_source(
    paths: &ToolPaths,
    url: &str,
    use_cookies: bool,
) -> Result<DesktopSourceAnalysis, AnalyzeError> {
    probe_unknown_source_cancellable(paths, url, use_cookies, &AtomicBool::new(false))
}

fn probe_unknown_source_cancellable(
    paths: &ToolPaths,
    url: &str,
    use_cookies: bool,
    cancel: &AtomicBool,
) -> Result<DesktopSourceAnalysis, AnalyzeError> {
    let (mut command, command_line) = paths
        .prepare_source_probe_command(url, use_cookies)
        .map_err(|message| AnalyzeError {
            message,
            command_line: None,
        })?;
    let failure = |message: String| AnalyzeError {
        message,
        command_line: Some(command_line.clone()),
    };
    let output = super::process_guard::run_tracked_command_output_cancellable(
        &mut command,
        "yt-dlp source probe",
        cancel,
    )
    .map_err(|error| failure(format!("Could not start source probe: {error}")))?;
    if !output.status.success() {
        return Err(failure(format!(
            "yt-dlp source probe failed: {}",
            super::humanize_yt_dlp_error(String::from_utf8_lossy(&output.stderr).trim())
        )));
    }
    let json: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| failure(format!("Could not parse source probe: {error}")))?;
    if matches!(json["_type"].as_str(), Some("playlist" | "multi_video")) {
        Ok(DesktopSourceAnalysis::Playlist)
    } else if json["formats"]
        .as_array()
        .is_some_and(|formats| !formats.is_empty())
    {
        // Flat-playlist affects entries only. A single-video probe is already a
        // full usable result, so do not fetch it again.
        Ok(DesktopSourceAnalysis::Single(AnalyzeOutput {
            json,
            command_line,
        }))
    } else {
        Err(failure(
            "Source probe did not return a playlist or usable single-video formats.".into(),
        ))
    }
}
