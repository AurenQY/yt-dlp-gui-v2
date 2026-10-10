//! Gallery metadata pipes: notify on post-after, before the extractor sleeps or
//! exits. Concurrently drain bounded stderr; unfinished posts never advance UI.
use super::gallery_dl::{
    GALLERY_POST_COMPLETED_MARKER, PreparedGalleryDlDownload, build_gallery_media_item,
    build_gallery_post, parse_gallery_dl_download_event,
};
use crate::domain::{GalleryAnalysis, GalleryPost};
use std::{
    io::{BufRead, BufReader, Read},
    process::Stdio,
    thread,
};

pub(super) fn read_posts(
    mut prepared: PreparedGalleryDlDownload,
    url: &str,
    retain_empty: bool,
    mut on_post: impl FnMut(&GalleryAnalysis, usize) -> Result<(), String>,
) -> Result<GalleryAnalysis, String> {
    let mut diagnostic = super::operation_diagnostics::OperationSpan::start(
        "gallery.metadata.stream",
        serde_json::json!({"command": super::operation_diagnostics::command_context(&prepared.command)}),
    );
    prepared
        .command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = prepared
        .command
        .spawn()
        .map_err(|e| format!("Could not start gallery-dl: {e}"))?;
    let _tracked = super::track_child_process(&child, "gallery-dl browser metadata stream");
    let stdout = child.stdout.take().ok_or("Missing gallery-dl stdout")?;
    let mut stderr = child.stderr.take().ok_or("Missing gallery-dl stderr")?;
    let mut analysis = GalleryAnalysis {
        source_url: url.to_owned(),
        ..Default::default()
    };
    let mut completed_posts = 0;
    let (result, status, detail) = thread::scope(|scope| {
        let error_reader = scope.spawn(move || -> Result<Vec<u8>, String> {
            let mut tail = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let n = stderr.read(&mut buffer).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                tail.extend_from_slice(&buffer[..n]);
                if tail.len() > 16384 {
                    tail.drain(..tail.len() - 16384);
                }
            }
            Ok(tail)
        });
        let result = (|| -> Result<(), String> {
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            let mut post_index = None;
            loop {
                line.clear();
                if reader
                    .read_until(b'\n', &mut line)
                    .map_err(|e| e.to_string())?
                    == 0
                {
                    break;
                }
                let text = String::from_utf8_lossy(&line);
                if text.trim() == GALLERY_POST_COMPLETED_MARKER {
                    if retain_empty && post_index.is_none() {
                        analysis.posts.push(GalleryPost::default());
                        post_index = Some(analysis.posts.len() - 1);
                    }
                    if let Some(index) = post_index.take() {
                        super::operation_diagnostics::record(
                            "gallery.post.ready",
                            serde_json::json!({"postIndex": index, "mediaCount": analysis.media.len()}),
                        );
                        on_post(&analysis, index)?;
                        completed_posts += 1;
                    }
                    continue;
                }
                let Some(event) = parse_gallery_dl_download_event(&text) else {
                    continue;
                };
                let event = event?;
                if event.artifact.media_url.is_empty() {
                    return Err("gallery-dl browser metadata did not contain a media URL.".into());
                }
                let index = *post_index.get_or_insert_with(|| {
                    analysis
                        .posts
                        .push(build_gallery_post(event.metadata.as_object()));
                    analysis.posts.len() - 1
                });
                let mut media = build_gallery_media_item(
                    Some(index),
                    url,
                    &event.artifact.media_url,
                    event.metadata.as_object(),
                );
                media.archive_key = event.artifact.archive_key;
                media.browser_fallback_urls = event.browser_fallback_urls;
                media.browser_relative_path =
                    Some(super::browser_gallery_path::gallery_browser_relative_path(
                        &event.artifact.path,
                        &prepared.destination,
                    )?);
                analysis.media.push(media);
            }
            if post_index.is_some() {
                return Err("gallery-dl ended before completing a post; retry from the last completed post.".into());
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = child.kill();
        }
        let status = child.wait();
        if status.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let detail = error_reader
            .join()
            .map_err(|_| "gallery-dl stderr reader failed".to_owned())
            .and_then(|r| r);
        (result, status, detail)
    });
    let error_text = detail
        .as_ref()
        .map(|bytes| String::from_utf8_lossy(bytes).to_ascii_lowercase())
        .unwrap_or_default();
    diagnostic.finish(if result.is_ok() && status.as_ref().is_ok_and(|s| s.success()) { "success" } else { "failed" }, serde_json::json!({
        "exitCode": status.as_ref().ok().and_then(|s| s.code()),
        "postCount": analysis.posts.len(), "completedPostCount": completed_posts,
        "mediaCount": analysis.media.len(), "stderrBytes": detail.as_ref().map(Vec::len).unwrap_or(0),
        "projectionFailed": result.is_err(),
        "http403": error_text.contains("403 forbidden") || error_text.contains("http error 403"),
        "http429": error_text.contains("429 too many") || error_text.contains("http error 429"),
        "javascriptChallenge": error_text.contains("javascript challenge"),
        "extractorError": error_text.contains("[error]"),
    }));
    result?;
    let status = status.map_err(|e| e.to_string())?;
    let detail = detail?;
    if !status.success() {
        return Err(format!(
            "gallery-dl metadata failed: {}",
            String::from_utf8_lossy(&detail).trim()
        ));
    }
    Ok(analysis)
}
