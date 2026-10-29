//! Bounded source discovery followed by the desktop's existing full-format
//! analysis. Native presentation must not maintain a second extraction command.
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};

use super::{
    AnalyzeError, ToolPaths, operation_diagnostics, playlist_entry_url, run_tracked_command_output,
};

pub(crate) struct PlaylistAnalysisSummary {
    pub(crate) item_count: usize,
    pub(crate) truncated: bool,
    pub(crate) warning: Option<String>,
    pub(crate) command_line: String,
}

pub(crate) fn analyze_playlist_formats(
    tools: &ToolPaths,
    url: &str,
    use_cookies: bool,
    item_limit: usize,
    cancelled: &AtomicBool,
    mut on_item: impl FnMut(Value) -> Result<(), String>,
) -> Result<PlaylistAnalysisSummary, AnalyzeError> {
    let tools = tools
        .bind_protected_config()
        .map_err(|message| AnalyzeError {
            message,
            command_line: None,
        })?;
    let limit = item_limit.max(1);
    let mut command = tools
        .prepare_playlist_discovery_command(url, use_cookies, Some(limit.saturating_add(1)))
        .map_err(|message| AnalyzeError {
            message,
            command_line: None,
        })?;
    let command_line =
        super::tools::format_command_line(std::path::Path::new(command.get_program()), &command);
    let failure = |message: String| AnalyzeError {
        message,
        command_line: Some(command_line.clone()),
    };
    let mut span = operation_diagnostics::OperationSpan::start(
        "yt.playlist.formats",
        json!({"itemLimit": limit, "workflow": "desktop-discovery-then-single"}),
    );
    if cancelled.load(Ordering::Acquire) {
        return Err(failure("Playlist analysis was cancelled.".into()));
    }
    let discovery = run_tracked_command_output(&mut command, "yt-dlp playlist discovery")
        .map_err(|error| failure(format!("Could not start playlist discovery: {error}")))?;
    if cancelled.load(Ordering::Acquire) {
        return Err(failure("Playlist analysis was cancelled.".into()));
    }
    let mut entries = Vec::new();
    for line in discovery.stdout.split(|byte| *byte == b'\n') {
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let entry: Value = serde_json::from_slice(line)
            .map_err(|error| failure(format!("Could not parse playlist discovery: {error}")))?;
        entries.push(entry);
    }
    let discovery_error = (!discovery.status.success() || entries.is_empty()).then(|| {
        let detail = String::from_utf8_lossy(&discovery.stderr);
        if detail.trim().is_empty() {
            "Playlist discovery did not return usable entries.".to_owned()
        } else {
            format!("yt-dlp playlist discovery failed: {}", detail.trim())
        }
    });
    if entries.is_empty() {
        span.finish("failed", json!({"discoveredCount": 0, "itemCount": 0}));
        return Err(failure(discovery_error.unwrap()));
    }

    let discovered = entries.len();
    let mut truncated = discovered > limit;
    let mut errors = discovery_error.into_iter().collect::<Vec<_>>();
    let mut delivered = 0;
    for entry in entries.into_iter().take(limit) {
        if cancelled.load(Ordering::Acquire) {
            return Err(failure("Playlist analysis was cancelled.".into()));
        }
        let Some(source) = playlist_entry_url(&entry) else {
            errors.push("Playlist entry did not provide a source URL.".into());
            continue;
        };
        // Identical complete-format operation to a desktop imported queue item;
        // the discovery entry is not falsely presented as downloadable formats.
        let output = match tools.analyze_url_detailed(&source, use_cookies) {
            Ok(output) => output,
            Err(error) => {
                errors.push(error.message);
                continue;
            }
        };
        if cancelled.load(Ordering::Acquire) {
            return Err(failure("Playlist analysis was cancelled.".into()));
        }
        let items = output
            .json
            .get("entries")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter(|item| !item.is_null())
                    .cloned()
                    .collect()
            })
            .unwrap_or_else(|| vec![output.json.clone()]);
        for item in items {
            if delivered >= limit {
                truncated = true;
                break;
            }
            if !item
                .get("formats")
                .and_then(Value::as_array)
                .is_some_and(|formats| !formats.is_empty())
            {
                errors.push("Playlist entry analysis did not return usable formats.".into());
                continue;
            }
            on_item(item).map_err(&failure)?;
            delivered += 1;
        }
        if delivered >= limit {
            break;
        }
    }
    let warning =
        (!errors.is_empty()).then(|| errors.into_iter().take(3).collect::<Vec<_>>().join("\n"));
    span.finish(
        if delivered == 0 {
            "failed"
        } else if warning.is_some() {
            "partial"
        } else {
            "completed"
        },
        json!({"discoveredCount": discovered, "itemCount": delivered, "truncated": truncated}),
    );
    if delivered == 0 {
        return Err(failure(warning.unwrap_or_else(|| {
            "Playlist analysis did not return usable formats.".into()
        })));
    }
    Ok(PlaylistAnalysisSummary {
        item_count: delivered,
        truncated,
        warning,
        command_line,
    })
}
