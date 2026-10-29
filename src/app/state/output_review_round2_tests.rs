//! Round 2 acceptance through actual dispatch, controlled localhost tools.
use super::review_round1::{cached_stream, finished, metadata, music_finished, new_state};
use super::*;

pub(super) fn run(root: &Path, tools: &ToolPaths, audio: &Path, video: &Path, server: &Server) {
    sidecars(root, tools, server);
    offline(root, tools, audio, server);
    lookup(root, tools, audio, server);
    cancellation(root, tools, server);
    assert!(video.is_file());
    println!(
        "R-OUT-001/005/006/007 and R-CACHE-006 controlled acceptance passed: {}",
        root.display()
    );
}

fn sidecars(root: &Path, tools: &ToolPaths, server: &Server) {
    let url = format!("{}/source.mp4", server.address);
    let mut info = metadata(tools, root, &url, "Sidecar source", false);
    info["subtitles"] =
        serde_json::json!({"en":[{"ext":"vtt","url":format!("{}/captions.vtt",server.address)}]});
    info["thumbnails"] =
        serde_json::json!([{"url":format!("{}/cover.jpg",server.address),"ext":"jpg"}]);
    fs::write(
        root.join("round1-input.json"),
        serde_json::to_vec(&info).unwrap(),
    )
    .unwrap();
    let output = Command::new(&tools.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=64x64",
            "-frames:v",
            "1",
        ])
        .arg(root.join("cover.jpg"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (name, embed) in [("round2-vtt-skip", false), ("round2-embed-only", true)] {
        let mut state = new_state(root, tools, name);
        state.config.output_existing_file = ExistingFilePolicy::Skip;
        let mut item = state.build_queue_item_from_url(&url);
        let mut data = VideoMetadata::empty_preview();
        data.title = "Sidecar source".into();
        let mut format = FormatOption::new("fixture", "MP4", MediaKind::Muxed);
        format.ext = "mp4".into();
        data.formats.push(format);
        data.subtitle_tracks
            .push(crate::domain::SubtitleOption::new(
                "en",
                SubtitleSource::Original,
                "en",
                "en",
                "English",
                None,
                None,
                "vtt",
                format!("{}/captions.vtt", server.address),
            ));
        item.metadata_state = MetadataState::Ready(data);
        item.selection.video_selector = "fixture".into();
        item.selection.subtitle_selector = "en".into();
        item.selection.subtitle_source = SubtitleSource::Original;
        item.selection.write_subtitles = true;
        item.selection.embed_subtitles = embed;
        item.selection.write_thumbnail = embed;
        item.selection.embed_thumbnail = embed;
        item.selection.file_name = "Literal sidecar".into();
        item.selection.name_origin = Some(crate::domain::OutputNameOrigin::Explicit);
        let id = item.id;
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        for suffix in ["en.srt", "en.vtt", "jpg"] {
            fs::write(
                dir.join(format!("Literal sidecar.{suffix}")),
                b"old sidecar sentinel",
            )
            .unwrap();
        }
        state.queue_items = vec![item];
        let before = server.media_get_count();
        state.start_download_task_at(id, true).unwrap();
        assert_eq!(
            state
                .last_dispatched_download
                .as_ref()
                .unwrap()
                .subtitle_ext,
            "vtt"
        );
        let result = finished(&state);
        if embed {
            let path = published(result);
            assert_eq!(path.file_name().unwrap(), "Literal sidecar.mp4");
            assert!(server.media_get_count() > before);
        } else {
            assert_eq!(
                result.disposition,
                OutputDisposition::SkippedExisting,
                "{:?}",
                result.result
            );
            assert_eq!(
                Path::new(&result.result.unwrap()).file_name().unwrap(),
                "Literal sidecar.en.srt"
            );
            assert_eq!(server.media_get_count(), before);
        }
        for suffix in ["en.srt", "en.vtt", "jpg"] {
            assert_eq!(
                fs::read(dir.join(format!("Literal sidecar.{suffix}"))).unwrap(),
                b"old sidecar sentinel"
            );
        }
    }
    let mut external = tools.clone();
    let file = root.join("round2-external.conf");
    external.yt_dlp_config = file.display().to_string();
    for option in [
        "--write-description",
        "--write-info-json",
        "--write-thumbnail",
        "--write-subs",
        "--embed-thumbnail",
        "--keep-video",
        "--all-subs",
        "--write-automatic-subs",
        "--remux-video=mkv",
        "--extract-audio",
        "--split-chapters",
    ] {
        fs::write(&file, format!("{option}\n")).unwrap();
        let before = server.media_get_count();
        let result = download(
            &external,
            request(&url, &root.join("round2-external"), "External"),
        );
        assert_eq!(result.disposition, OutputDisposition::Failed);
        assert!(
            result
                .result
                .unwrap_err()
                .contains(option.split('=').next().unwrap())
        );
        assert_eq!(server.media_get_count(), before);
    }
    println!(
        "R-OUT-001/005: actual VTT dispatch skips final SRT before media; embed-only ignores external sidecars; unplannable external sidecars reject before transfer"
    );
}

fn choice(format: MusicDownloadFormat) -> MusicDownloadChoice {
    MusicDownloadChoice {
        mode: MusicDownloadMode::Unified,
        unified_format: format,
        embed_cover: false,
        write_tags: false,
        ..Default::default()
    }
}

fn offline(root: &Path, tools: &ToolPaths, audio: &Path, server: &Server) {
    // A loopback endpoint that cannot be contacted; no source config can make
    // a metadata query succeed. A schema-compatible complete cache is retained.
    let source = "http://127.0.0.1:1/offline-track.wav";
    let value = metadata(tools, root, source, "Trusted cached 空间 100%", true);
    let seed = music_stream_seed_from_json(&value, source).unwrap();
    let config = Path::new(&tools.yt_dlp)
        .parent()
        .unwrap()
        .join("yt-dlp.conf");
    fs::remove_file(&config).unwrap();
    fs::write(
        &config,
        "--retries 0\n--extractor-retries 0\n--socket-timeout 1\n",
    )
    .unwrap();
    for (label, format) in [
        ("copy", MusicDownloadFormat::Wav),
        ("convert", MusicDownloadFormat::Flac),
    ] {
        let mut state = new_state(root, tools, &format!("round2-offline-{label}"));
        let cache = cached_stream(&state, &seed, audio);
        let original = fs::read(cache.join("manifest.yaml")).unwrap();
        let item = state.build_queue_item_from_url(source);
        let snapshot =
            crate::domain::audio_queue_snapshot::AudioQueueItemSnapshot::from_item(&item);
        let restored = state
            .queue_item_from_audio_playlist_snapshot(snapshot)
            .unwrap();
        assert!(restored.music_naming_metadata.is_none());
        assert_eq!(restored.title, "Trusted cached 空间 100%");
        let id = restored.id;
        state.queue_items = vec![restored];
        let context = state
            .output_request_for_item(&state.queue_items[0], false)
            .context;
        assert!(!context.resolved && !state.config.output_preferences().needs_metadata(&context));
        let before = server.media_get_count();
        state
            .start_music_download_task_at(id, choice(format))
            .unwrap();
        let (kind, disposition, result) = music_finished(&state);
        assert_eq!(disposition, OutputDisposition::Published, "{result:?}");
        assert_eq!(
            kind,
            if label == "copy" {
                MusicDownloadSourceKind::CacheCopy
            } else {
                MusicDownloadSourceKind::CacheConvert
            }
        );
        let path = PathBuf::from(result.unwrap());
        assert_eq!(
            path.file_name().unwrap(),
            format!(
                "Trusted cached 空间 100%.{}",
                if label == "copy" { "wav" } else { "flac" }
            )
            .as_str()
        );
        if label == "copy" {
            assert_eq!(fs::read(&path).unwrap(), fs::read(audio).unwrap());
        }
        assert_eq!(fs::read(cache.join("manifest.yaml")).unwrap(), original);
        assert_eq!(server.media_get_count(), before);
        // Fields unknown in the old schema are not declared absent. A rule
        // that needs uploader/date still visibly fails if resolution is offline.
        let mut required = state.output_request_for_item(&state.queue_items[0], false);
        required.preferences.naming = OutputNamingMode::Custom;
        required.preferences.template = "{uploader} {upload_date}".into();
        assert!(required.preferences.needs_metadata(&required.context));
        let error = tools
            .music_output_destination(
                &required,
                source,
                &root.join("unknown-fields"),
                "unused",
                false,
            )
            .unwrap_err();
        assert!(
            error.contains("probe") || error.contains("analysis"),
            "{error}"
        );
        // Explicit names require no metadata even with a conflicting rule.
        required.automatic = false;
        required.literal_name = "Offline explicit".into();
        assert_eq!(
            tools
                .music_output_destination(
                    &required,
                    source,
                    &root.join("explicit"),
                    "Offline explicit",
                    false
                )
                .unwrap()
                .1,
            "Offline explicit"
        );
    }
    println!(
        "R-OUT-007: restored real snapshot + unchanged complete-cache schema, offline Title copy/conversion and explicit priority publish locally; unknown fields remain unresolved"
    );
}

fn lookup(root: &Path, tools: &ToolPaths, audio: &Path, server: &Server) {
    let mut state = new_state(root, tools, "round2-cache-lookup");
    let source = format!("{}/cache-only.wav", server.address);
    let info = metadata(tools, root, &source, "Cache owner", true);
    let seed = music_stream_seed_from_json(&info, &source).unwrap();
    let owned = cached_stream(&state, &seed, audio);
    let mut value: serde_json::Value =
        serde_yaml::from_slice(&fs::read(owned.join("manifest.yaml")).unwrap()).unwrap();
    value["updated_unix_seconds"] = serde_json::json!(1);
    fs::write(
        owned.join("manifest.yaml"),
        serde_yaml::to_string(&value).unwrap(),
    )
    .unwrap();
    let unknown = storage::cache_directory("audio")
        .unwrap()
        .join("unknown-sentinel");
    fs::create_dir_all(&unknown).unwrap();
    fs::write(unknown.join("manifest.yaml"), b"{}\n").unwrap();
    fs::write(unknown.join("keep.txt"), b"unknown preserve").unwrap();
    let item = state.build_queue_item_from_url(&source);
    let active = storage::use_cache().unwrap();
    assert!(state.complete_music_cache_hit_for_item(&item).is_none());
    assert!(owned.is_dir() && unknown.join("keep.txt").is_file());
    assert!(storage::exclusive_cleanup().is_err());
    drop(active);
    // Junctions require no symlink privilege; all targets are isolated project
    // sentinels. Remove junctions with remove_dir only, never recursively.
    let audio_root = state.music_stream_cache_root();
    let moved = root.join("round2-audio-held");
    fs::rename(&audio_root, &moved).unwrap();
    let external = root.join("round2-external-sentinel");
    fs::create_dir_all(external.join("stale")).unwrap();
    fs::write(external.join("stale/manifest.yaml"), b"{}\n").unwrap();
    fs::write(external.join("stale/keep.txt"), b"external preserve").unwrap();
    junction(&audio_root, &external);
    assert!(state.complete_music_cache_hit_for_item(&item).is_none());
    assert_eq!(
        fs::read(external.join("stale/keep.txt")).unwrap(),
        b"external preserve"
    );
    fs::remove_dir(&audio_root).unwrap();
    fs::rename(&moved, &audio_root).unwrap();
    let link = audio_root.join("linked-child");
    junction(&link, &external);
    assert!(state.complete_music_cache_hit_for_item(&item).is_none());
    fs::remove_dir(&link).unwrap();
    let _exclusive = storage::exclusive_cleanup().unwrap();
    let summary = remove_expired_music_cache_dirs(&audio_root).unwrap();
    assert_eq!(summary.entries, 1);
    assert!(!owned.exists());
    assert_eq!(
        fs::read(unknown.join("keep.txt")).unwrap(),
        b"unknown preserve"
    );
    assert_eq!(
        fs::read(external.join("stale/keep.txt")).unwrap(),
        b"external preserve"
    );
    println!(
        "R-CACHE-006: real lookup retains active stale/unknown entries; parent and child junctions rejected; guarded expiry removes only stale owned entry"
    );
}

fn junction(link: &Path, target: &Path) {
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link.to_string_lossy().replace('/', "\\"))
        .arg(target.to_string_lossy().replace('/', "\\"))
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "junction unavailable: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

fn wait_blocked(root: &Path) -> Vec<u32> {
    let start = Instant::now();
    while !root.join("metadata-blocked.txt").exists() {
        assert!(start.elapsed() < Duration::from_secs(12));
        thread::sleep(Duration::from_millis(25));
    }
    let pids = crate::infrastructure::tracked_test_pids();
    assert!(!pids.is_empty());
    assert!(storage::exclusive_cleanup().is_err());
    pids
}
fn settled(root: &Path, pids: Vec<u32>, start: Instant) {
    assert!(
        start.elapsed() < Duration::from_secs(6),
        "cancel did not settle promptly"
    );
    assert!(crate::infrastructure::tracked_test_pids().is_empty());
    for pid in pids {
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .unwrap();
        assert!(!String::from_utf8_lossy(&out.stdout).contains(&pid.to_string()));
    }
    let _cleanup = storage::exclusive_cleanup().unwrap();
    fs::remove_file(root.join("metadata-blocked.txt")).unwrap();
}
fn cancellation(root: &Path, tools: &ToolPaths, server: &Server) {
    let config = Path::new(&tools.yt_dlp)
        .parent()
        .unwrap()
        .join("yt-dlp.conf");
    fs::remove_file(&config).unwrap();
    for route in ["direct", "music", "original"] {
        let mut state = new_state(root, tools, &format!("round2-cancel-{route}"));
        let url = format!("{}/metadata-block-{route}", server.address);
        let mut item = state.build_queue_item_from_url(&url);
        // Original route bypasses naming lookup, isolating its extension probe.
        if route == "original" {
            item.selection.name_origin = Some(crate::domain::OutputNameOrigin::Explicit);
            item.selection.file_name = "Original".into();
        }
        let id = item.id;
        state.queue_items = vec![item];
        let before = server.media_get_count();
        if route == "direct" {
            state.start_download_task_at(id, true).unwrap();
        } else {
            let mut selected = choice(MusicDownloadFormat::Wav);
            if route == "original" {
                selected.mode = MusicDownloadMode::Original;
            }
            state.start_music_download_task_at(id, selected).unwrap();
        }
        let pids = wait_blocked(root);
        let start = Instant::now();
        state.cancel_item_download(id);
        if route == "direct" {
            assert_eq!(finished(&state).disposition, OutputDisposition::Cancelled);
        } else {
            assert_eq!(music_finished(&state).1, OutputDisposition::Cancelled);
        }
        settled(root, pids, start);
        println!("R-OUT-006 {route}: cancelled producer and readers settled");
        assert_eq!(server.media_get_count(), before);
        assert!(
            !root
                .join(format!("round2-cancel-{route}/Original.wav"))
                .exists()
        );
    }
    let mut config =
        AppConfig::default().with_fixture_file(crate::infrastructure::runtime_config_file_path());
    config.download_dir = root.join("round2-native-cancel").display().to_string();
    fs::create_dir_all(&config.download_dir).unwrap();
    config.yt_dlp_path = tools.yt_dlp.clone();
    config.ffmpeg_path = tools.ffmpeg.clone();
    config.use_browser_cookies = false;
    config.proxy_enabled = true;
    config.proxy_url = server.address.clone();
    config.save().unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let before = server.media_get_count();
    let worker = thread::spawn(move || {
        crate::infrastructure::browser_native_messaging::output_review_native_cancel(
            &serde_json::json!({"url":"http://output-fixture.invalid/metadata-block-native","videoFormatId":"best","videoIsMuxed":true,"useAria2":false,"thumbnailMode":"off","subtitleMode":"off","chapterMode":"off"}),
            &flag,
        )
    });
    thread::sleep(Duration::from_millis(800));
    if worker.is_finished() {
        panic!(
            "Native fixture returned before blocked metadata: {:?}",
            worker.join().unwrap()
        );
    }
    let pids = wait_blocked(root);
    let start = Instant::now();
    cancelled.store(true, Ordering::Release);
    assert!(
        worker
            .join()
            .unwrap()
            .unwrap_err()
            .to_lowercase()
            .contains("cancel")
    );
    settled(root, pids, start);
    assert_eq!(server.media_get_count(), before);
    println!(
        "R-OUT-006: Stop during actual direct/music metadata and Original extension/native probes settles child/pipes promptly; zero media GETs and exclusive cleanup succeeds"
    );
}
