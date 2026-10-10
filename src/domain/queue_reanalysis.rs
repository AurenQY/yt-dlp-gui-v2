//! Pure restored-intent checks: never replace a saved selection silently.
use super::{DownloadSelection, MediaKind, MetadataState, QueueItem, VideoMetadata};
use serde_json::Value;

pub(crate) fn initialize_missing_selectors(
    selection: &mut DownloadSelection,
    video: &str,
    audio: &str,
) {
    // An entirely unanalysed seed needs defaults. An audio-only or other existing
    // choice must retain intentional empty selectors as well as nonempty ones.
    if selection.video_selector.is_empty() && selection.audio_selector.is_empty() {
        selection.video_selector = video.to_owned();
        selection.audio_selector = audio.to_owned();
    }
}

pub(crate) fn install_refreshed_video_metadata(
    item: &mut QueueItem,
    metadata: &VideoMetadata,
    default_video: &str,
    default_audio: &str,
) {
    item.title = metadata.title.clone();
    item.thumbnail_url = metadata.thumbnail_url.clone();
    item.duration_text = metadata.duration_text.clone();
    item.metadata_state = MetadataState::Ready(metadata.clone());
    initialize_missing_selectors(&mut item.selection, default_video, default_audio);
    item.needs_analysis_refresh = false;
    // Filename/container/subtitle/options/ranges are user intent, not analyzer defaults.
}

pub(crate) fn select_reanalyzed_video(
    json: Value,
    index: Option<usize>,
    expected_id: Option<&str>,
) -> Result<Value, String> {
    let video = if let Some(entries) = json.get("entries").and_then(Value::as_array) {
        let index = index.ok_or("Analysis returned a playlist instead of the saved video.")?;
        entries
            .iter()
            .find(|entry| entry.get("playlist_index").and_then(Value::as_u64) == Some(index as u64))
            .cloned()
            .ok_or("The saved playlist item no longer exists.")?
    } else {
        json
    };
    if let Some(expected) = expected_id {
        if video.get("id").and_then(Value::as_str) != Some(expected) {
            return Err("The playlist item changed. Add the intended video again.".into());
        }
    }
    if !video
        .get("formats")
        .and_then(Value::as_array)
        .is_some_and(|formats| !formats.is_empty())
    {
        return Err("The saved video has no available formats.".into());
    }
    Ok(video)
}

pub(crate) fn validate_saved_selection(
    selection: &DownloadSelection,
    metadata: &VideoMetadata,
) -> Result<(), String> {
    for (name, id, kinds) in [
        (
            "video",
            selection.video_selector.as_str(),
            &[MediaKind::Video, MediaKind::Muxed][..],
        ),
        (
            "audio",
            selection.audio_selector.as_str(),
            &[MediaKind::Audio, MediaKind::Muxed][..],
        ),
    ] {
        if !id.is_empty()
            && !metadata
                .formats
                .iter()
                .any(|f| f.id == id && kinds.contains(&f.kind))
        {
            return Err(format!(
                "Saved {name} format '{id}' is unavailable. Select a format again."
            ));
        }
    }
    if !selection.subtitle_selector.is_empty()
        && !metadata
            .subtitle_tracks
            .iter()
            .any(|s| s.id == selection.subtitle_selector && s.source == selection.subtitle_source)
    {
        return Err("The saved subtitle is unavailable. Select a subtitle again.".into());
    }
    if metadata.duration_millis.is_some_and(|duration| {
        selection
            .download_range
            .custom_time_ranges()
            .iter()
            .any(|r| r.end_millis() > duration)
    }) {
        return Err(
            "The saved download range exceeds the current video duration. Select the range again."
                .into(),
        );
    }
    Ok(())
}
