//! Parsed DataJob outcomes, not merely a successful gallery-dl exit code.
//! Keep source output and credentials in memory; persist only counts and flags.
use std::process::Output;

use crate::domain::GalleryAnalysis;

pub(super) fn record_metadata_result(
    resolves_children: bool,
    output: &Output,
    parsed: Option<&Result<GalleryAnalysis, String>>,
) {
    let analysis = parsed.and_then(|result| result.as_ref().ok());
    let parse_failed = parsed.is_some_and(Result::is_err);
    let detail = format!(
        "{} {}",
        String::from_utf8_lossy(&output.stderr),
        parsed
            .and_then(|result| result.as_ref().err())
            .map(String::as_str)
            .unwrap_or_default(),
    )
    .to_ascii_lowercase();
    super::super::operation_diagnostics::record(
        "gallery.metadata.resolved",
        serde_json::json!({
            "resolvesChildren": resolves_children,
            "status": if !output.status.success() { "process-failed" }
                else if parse_failed { "parse-failed" } else { "success" },
            "exitCode": output.status.code(),
            "postCount": analysis.map(|value| value.posts.len()).unwrap_or(0),
            "mediaCount": analysis.map(|value| value.media.len()).unwrap_or(0),
            "queuedCount": analysis.map(|value| value.queued_urls.len()).unwrap_or(0),
            "parseFailed": parse_failed,
            "http403": detail.contains("403 forbidden") || detail.contains("http error 403"),
            "http429": detail.contains("429 too many") || detail.contains("http error 429"),
            "javascriptChallenge": detail.contains("javascript challenge"),
            "extractorError": detail.contains("[error]"),
        }),
    );
}
