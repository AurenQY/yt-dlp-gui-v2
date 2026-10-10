//! Bound metadata discovery before gallery-dl emits post-directory events.
//! General --post-range alone cannot stop eager TikTok account enumeration.
use std::process::Command;

pub(super) const ANALYSIS_POST_LIMIT: u32 = 2;

/// The registry proves the actual upstream Dispatch helper and target tuple.
/// Names, URL shapes or a small analysis sample are not delegation evidence.
pub(super) fn is_proven_root_dispatch(
    metadata: Option<&super::super::extractor_index::ExtractorMetadata>,
) -> bool {
    metadata.is_some_and(|metadata| {
        metadata
            .details
            .get("basis")
            .and_then(serde_json::Value::as_str)
            == Some("upstream-dispatch")
            && metadata
                .details
                .get("postRange")
                .and_then(serde_json::Value::as_str)
                == Some("delegated")
    })
}

pub(super) fn append_profile_feed_policy(command: &mut Command) {
    // TikTok's bare profile dispatches avatar + posts by default. A profile is
    // a post feed in v2, so analysis, continuation and download must agree on
    // the same single feed. Explicit /avatar URLs are unaffected.
    command.arg("-o").arg("extractor.tiktok.user.include=posts");
}

pub(super) fn append_post_window(
    command: &mut Command,
    start: u32,
    end: u32,
    has_post_filter: bool,
    root_extractor_key: Option<&str>,
) {
    // These two exact upstream roots use TiktokItemListRequest.generate_urls
    // (or yt-dlp playlist_items) before fetching any post detail. The user
    // Dispatch is posts-only under append_profile_feed_policy. Do not infer
    // this capability from a hostname, category or other TikTok class name.
    let slice_before_details = !has_post_filter
        && start > 1
        && end >= start
        && matches!(
            root_extractor_key,
            Some(
                "gallery_dl.extractor.tiktok.TiktokUserExtractor"
                    | "gallery_dl.extractor.tiktok.TiktokPostsExtractor"
            )
        );
    let (job_start, job_end) = if slice_before_details {
        // The eager slice restarts Directory ordinals at one. Keeping the
        // original start here would skip the same prefix a second time.
        (1, end - start + 1)
    } else {
        (start, end)
    };
    command
        .arg("--post-range")
        .arg(format!("{job_start}-{job_end}"));
    if !has_post_filter {
        // Date filters can reject arbitrarily many prefix posts, so they must
        // not inherit an unfiltered ordinal cap.
        let eager_start = if slice_before_details { start } else { 1 };
        command
            .arg("-o")
            .arg(format!("extractor.tiktok.tiktok-range={eager_start}-{end}"));
    }
    super::super::operation_diagnostics::record(
        "gallery.post.window",
        serde_json::json!({
            "sourceStart": start, "sourceEnd": end,
            "jobStart": job_start, "jobEnd": job_end,
            "sliceBeforeDetails": slice_before_details, "hasPostFilter": has_post_filter,
        }),
    );
}
