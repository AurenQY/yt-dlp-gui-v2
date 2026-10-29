//! Round 3: standalone outputs, chapter fallback and cancelled FFmpeg postpasses.
use super::review_round1::{cached_stream, finished, metadata, new_state};
use super::*;

pub(super) fn run(root: &Path, tools: &ToolPaths, audio: &Path, server: &Server) {
    standalone_subtitles(root, tools, server);
    chapter_fallback(root, tools, audio, server);
    postpasses(root, tools, audio, server);
    println!(
        "R-OUT-001/005/008/009 acceptance passed: {}",
        root.display()
    );
}

pub(super) fn music_result(
    state: &mut AppState,
) -> (
    MusicDownloadSourceKind,
    OutputDisposition,
    Result<String, String>,
    bool,
    bool,
) {
    loop {
        let event = state
            .music_download_event_rx
            .recv_timeout(Duration::from_secs(40))
            .unwrap();
        let result = if let MusicDownloadEvent::Finished {
            source_kind,
            disposition,
            result,
            split_chapters_requested,
            chapters_split,
            ..
        } = &event
        {
            Some((
                *source_kind,
                *disposition,
                result.clone(),
                *split_chapters_requested,
                *chapters_split,
            ))
        } else {
            None
        };
        state.apply_music_download_event(event);
        if let Some(result) = result {
            return result;
        }
    }
}

fn subtitle_item(state: &mut AppState, url: &str, server: &Server) -> QueueItemId {
    let mut item = state.build_queue_item_from_url(url);
    let mut data = VideoMetadata::empty_preview();
    data.title = "Subtitle source".into();
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
    item.selection.subtitle_selector = "en".into();
    item.selection.subtitle_source = SubtitleSource::Original;
    let id = item.id;
    state.queue_items = vec![item];
    id
}

fn standalone_subtitles(root: &Path, tools: &ToolPaths, server: &Server) {
    let url = format!("{}/subtitle-source.mp4", server.address);
    let mut info = metadata(tools, root, &url, "Subtitle source", false);
    info["subtitles"] =
        serde_json::json!({"en":[{"ext":"vtt","url":format!("{}/captions.vtt",server.address)}]});
    fs::write(
        root.join("round1-input.json"),
        serde_json::to_vec(&info).unwrap(),
    )
    .unwrap();
    for (name, policy) in [
        ("round3-subtitle-skip", ExistingFilePolicy::Skip),
        ("round3-subtitle-keep", ExistingFilePolicy::KeepBoth),
    ] {
        let folder = root.join(name);
        fs::create_dir_all(&folder).unwrap();
        let neighbor = folder.join("Clip.en.srt");
        fs::write(&neighbor, b"unrelated language neighbor").unwrap();
        for repeat in [false, true] {
            let mut state = new_state(root, tools, name);
            state.config.output_existing_file = policy;
            let id = subtitle_item(&mut state, &url, server);
            let before = fs::read_to_string(root.join("http-requests.jsonl"))
                .unwrap_or_default()
                .lines()
                .count();
            let base_before = folder
                .join("Clip.srt")
                .is_file()
                .then(|| crate::infrastructure::sha256_hex_file(&folder.join("Clip.srt")).unwrap());
            state
                .start_item_export(
                    id,
                    DownloadTargetKind::Subtitle,
                    folder.join("Clip.srt").display().to_string(),
                )
                .unwrap();
            let result = finished(&state);
            let skip = repeat && policy == ExistingFilePolicy::Skip;
            assert_eq!(
                result.disposition,
                if skip {
                    OutputDisposition::SkippedExisting
                } else {
                    OutputDisposition::Published
                },
                "{:?}",
                result.result
            );
            let path = PathBuf::from(result.result.unwrap());
            assert_eq!(
                path.file_name().unwrap(),
                if repeat && !skip {
                    "Clip (2).srt"
                } else {
                    "Clip.srt"
                }
            );
            assert!(
                fs::read_to_string(&path)
                    .unwrap()
                    .contains("Example subtitle")
            );
            assert_eq!(fs::read(&neighbor).unwrap(), b"unrelated language neighbor");
            if let Some(hash) = base_before {
                assert_eq!(
                    crate::infrastructure::sha256_hex_file(&folder.join("Clip.srt")).unwrap(),
                    hash
                );
            }
            if skip {
                assert_eq!(
                    fs::read_to_string(root.join("http-requests.jsonl"))
                        .unwrap()
                        .lines()
                        .count(),
                    before
                );
            }
            fs::write(folder.join(format!("case-{repeat}.json")), serde_json::to_vec_pretty(&serde_json::json!({
                "policy":format!("{policy:?}"),"repeat":repeat,"path":path,
                "disposition":format!("{:?}", result.disposition),
                "http_requests_before":before,
                "http_requests_after":fs::read_to_string(root.join("http-requests.jsonl")).unwrap().lines().count()
            })).unwrap()).unwrap();
        }
    }
    println!(
        "R-OUT-001/005: actual standalone export publishes absent Clip.srt under Skip/KeepBoth despite Clip.en.srt; occupied Clip.srt skips before any caption/media request"
    );
}

pub(super) fn choice(format: MusicDownloadFormat) -> MusicDownloadChoice {
    MusicDownloadChoice {
        mode: MusicDownloadMode::Unified,
        unified_format: format,
        embed_cover: false,
        write_tags: false,
        ..Default::default()
    }
}

pub(super) fn music_item(state: &mut AppState, url: &str, title: &str) -> QueueItemId {
    let mut item = state.build_queue_item_from_url(url);
    let mut data = VideoMetadata::empty_preview();
    data.title = title.into();
    item.music_naming_metadata = Some(data);
    item.music_album_title = "Controlled album".into();
    let id = item.id;
    state.queue_items = vec![item];
    id
}

fn chapter_fallback(root: &Path, tools: &ToolPaths, audio: &Path, server: &Server) {
    for original in [false, true] {
        for case in [
            "skip-occupied",
            "skip-absent",
            "keep-occupied",
            "chapters",
            "skip-occupied-cached",
            "keep-occupied-cached",
        ] {
            let cached = case.ends_with("-cached");
            if cached && !original {
                continue;
            }
            let name = format!("round3-chapters-{original}-{case}");
            let url = format!("{}/audio.wav?{name}", server.address);
            let mut info = metadata(tools, root, &url, "Track", true);
            if case != "chapters" {
                info.as_object_mut().unwrap().remove("chapters");
            }
            fs::write(
                root.join("round1-input.json"),
                serde_json::to_vec(&info).unwrap(),
            )
            .unwrap();
            let mut state = new_state(root, tools, &name);
            let id = music_item(&mut state, &url, "Track");
            let cached_audio = if cached {
                let mp3 = root.join("round3-chapter-cache.mp3");
                if !mp3.is_file() {
                    let converted = Command::new(&tools.ffmpeg)
                        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
                        .arg(audio)
                        .args(["-c:a", "libmp3lame"])
                        .arg(&mp3)
                        .output()
                        .unwrap();
                    assert!(converted.status.success());
                }
                let mut cached_info = info.clone();
                cached_info["ext"] = "mp3".into();
                cached_info["format_id"] = "cached-mp3".into();
                cached_info["acodec"] = "mp3".into();
                let seed = music_stream_seed_from_json(&cached_info, &url).unwrap();
                let directory = cached_stream(&state, &seed, &mp3);
                state.restore_music_compact_cache_hit_if_available(id);
                assert_eq!(
                    state.complete_music_cache_media_path(&state.queue_items[0]),
                    Some(directory.join("audio.mp3"))
                );
                Some(directory.join("audio.mp3"))
            } else {
                None
            };
            let cached_hash = cached_audio
                .as_ref()
                .map(|path| crate::infrastructure::sha256_hex_file(path).unwrap());
            let folder = root.join(&name);
            fs::create_dir_all(&folder).unwrap();
            let ext = if original { "wav" } else { "mp3" };
            let target = folder.join(format!("Track.{ext}"));
            let occupied = case != "skip-absent";
            if occupied {
                fs::write(&target, b"old full track sentinel").unwrap();
            }
            state.config.output_existing_file = if case.starts_with("keep-occupied") {
                ExistingFilePolicy::KeepBoth
            } else {
                ExistingFilePolicy::Skip
            };
            let mut selected = choice(MusicDownloadFormat::Mp3);
            selected.save_mode = MusicDownloadSaveMode::SplitChapters;
            if original {
                selected.mode = MusicDownloadMode::Original;
            }
            let before = server.media_get_count();
            state.start_music_download_task_at(id, selected).unwrap();
            let (kind, disposition, result, split_requested, split_done) = music_result(&mut state);
            assert!(split_requested);
            assert_eq!(split_done, case == "chapters");
            assert_eq!(kind, MusicDownloadSourceKind::YtDlpDownload);
            let skip = case.starts_with("skip-occupied");
            assert_eq!(
                disposition,
                if skip {
                    OutputDisposition::SkippedExisting
                } else {
                    OutputDisposition::Published
                },
                "{name}: {result:?}"
            );
            let path = PathBuf::from(result.unwrap());
            if case == "chapters" {
                assert_eq!(path.file_name().unwrap(), "Track");
                assert!(path.is_dir());
                assert_eq!(fs::read_dir(&path).unwrap().count(), 2);
            } else {
                assert_eq!(
                    path.file_name().unwrap(),
                    format!(
                        "Track{}.{ext}",
                        if case.starts_with("keep-occupied") {
                            " (2)"
                        } else {
                            ""
                        }
                    )
                    .as_str()
                );
                assert!(path.is_file());
                assert!(!folder.join("Track").exists());
            }
            if skip {
                assert_eq!(server.media_get_count(), before);
            } else {
                assert!(server.media_get_count() > before);
            }
            if occupied {
                assert_eq!(fs::read(&target).unwrap(), b"old full track sentinel");
            }
            if let Some(path) = cached_audio {
                assert_eq!(
                    crate::infrastructure::sha256_hex_file(&path).unwrap(),
                    cached_hash.unwrap()
                );
            }
            if !skip && case != "chapters" {
                assert_eq!(
                    state.last_action,
                    state.ui_i18n_text_for_key("notification.no_chapters_saved_full_audio")
                );
            }
            fs::write(folder.join("case-result.json"), serde_json::to_vec_pretty(&serde_json::json!({
                "case":case,"original":original,"cached_mp3_with_online_wav":cached,"disposition":format!("{disposition:?}"),
                "path":path,"chapters_requested":split_requested,"chapters_split":split_done,
                "media_gets_before":before,"media_gets_after":server.media_get_count(),"feedback":state.last_action
            })).unwrap()).unwrap();
        }
    }
    println!(
        "R-OUT-008: actual chapter dispatch resolves no-chapters fallback before transfer; MP3 and Original/WAV occupied/absent/KeepBoth and directory-only chapter cases passed"
    );
}

fn postpasses(root: &Path, tools: &ToolPaths, audio: &Path, server: &Server) {
    let fixture = root.join("postpass-tools");
    fs::create_dir_all(&fixture).unwrap();
    let compiled = fixture.join("ffmpeg.exe");
    let result = Command::new("rustc")
        .args(["--edition=2024", "--crate-name", "output_postpass_fixture"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app/state/output_postpass_fixture.rs"))
        .arg("-o")
        .arg(&compiled)
        .output()
        .unwrap();
    fs::write(fixture.join("rustc.stdout.log"), &result.stdout).unwrap();
    fs::write(fixture.join("rustc.stderr.log"), &result.stderr).unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    fs::copy(&compiled, fixture.join("yt-dlp.exe")).unwrap();
    let mp3 = fixture.join("input.mp3");
    let result = Command::new(&tools.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(audio)
        .args(["-c:a", "libmp3lame"])
        .arg(&mp3)
        .output()
        .unwrap();
    assert!(result.status.success());
    let cover = fixture.join("cover.jpg");
    let result = Command::new(&tools.ffmpeg)
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
        .arg(&cover)
        .output()
        .unwrap();
    assert!(result.status.success());
    for (stage, cached) in [
        ("extension", false),
        ("cover", false),
        ("metadata", false),
        ("metadata", true),
    ] {
        for cancel in [true, false] {
            let name = format!("round3-postpass-{stage}-{cached}-{cancel}");
            let folder = root.join(&name);
            fs::create_dir_all(&folder).unwrap();
            let url = format!("{}/audio.wav?{name}", server.address);
            let mut fake = tools.clone();
            fake.yt_dlp = fixture.join("yt-dlp.exe").display().to_string();
            fake.ffmpeg = compiled.display().to_string();
            let mut state = new_state(root, &fake, &name);
            let id = music_item(&mut state, &url, "Protected track");
            if cached {
                let info = metadata(&fake, root, &url, "Protected track", true);
                let seed = music_stream_seed_from_json(&info, &url).unwrap();
                cached_stream(&state, &seed, audio);
                state.restore_music_compact_cache_hit_if_available(id);
            }
            let embed = matches!(stage, "extension" | "cover");
            if embed {
                let directory = state
                    .music_cache_cover_write_dir(&state.queue_items[0])
                    .unwrap();
                fs::create_dir_all(&directory).unwrap();
                fs::copy(&cover, directory.join("cover.jpg")).unwrap();
            }
            let ext = if cached { "wav" } else { "mp3" };
            let old = folder.join(format!("Protected track.{ext}"));
            fs::write(&old, b"existing output remains intact").unwrap();
            let source = if stage == "extension" || cached {
                audio
            } else {
                mp3.as_path()
            };
            let source_hash = crate::infrastructure::sha256_hex_file(source).unwrap();
            // Environment is confined to this isolated single-test child.
            for (key, value) in [
                ("OUTPUT_POSTPASS_ROOT", folder.display().to_string()),
                (
                    "OUTPUT_POSTPASS_BLOCK",
                    if cancel { stage.into() } else { "none".into() },
                ),
                ("OUTPUT_POSTPASS_REAL_YTDLP", tools.yt_dlp.clone()),
                ("OUTPUT_POSTPASS_REAL_FFMPEG", tools.ffmpeg.clone()),
                (
                    "OUTPUT_POSTPASS_ALLOWED_CACHE",
                    storage::paths()
                        .cache_root
                        .canonicalize()
                        .unwrap()
                        .display()
                        .to_string(),
                ),
                ("OUTPUT_POSTPASS_MEDIA", source.display().to_string()),
                (
                    "OUTPUT_POSTPASS_MEDIA_EXT",
                    if stage == "extension" || cached {
                        "wav".into()
                    } else {
                        "mp3".into()
                    },
                ),
            ] {
                unsafe {
                    std::env::set_var(key, value);
                }
            }
            let mut selected = choice(if cached {
                MusicDownloadFormat::Wav
            } else {
                MusicDownloadFormat::Mp3
            });
            selected.write_tags = true;
            selected.embed_cover = embed;
            state.start_music_download_task_at(id, selected).unwrap();
            let mut pids = Vec::new();
            let started = Instant::now();
            if cancel {
                while !folder.join("active-pids.txt").exists() {
                    assert!(
                        started.elapsed() < Duration::from_secs(12),
                        "{name}: no postpass marker"
                    );
                    thread::sleep(Duration::from_millis(25));
                }
                pids = fs::read_to_string(folder.join("active-pids.txt"))
                    .unwrap()
                    .lines()
                    .map(|s| s.parse::<u32>().unwrap())
                    .collect();
                assert!(storage::exclusive_cleanup().is_err());
                // The main media child is finished: Stop must reach the postpass.
                assert!(state.active_workflows.values().all(|workflow| {
                    workflow
                        .download_child
                        .as_ref()
                        .is_none_or(|child| child.lock().unwrap().is_none())
                }));
                state.cancel_item_download(id);
            }
            let cancelled_at = Instant::now();
            let (kind, disposition, result, _, _) = music_result(&mut state);
            assert_eq!(
                kind,
                if cached {
                    MusicDownloadSourceKind::CacheCopy
                } else {
                    MusicDownloadSourceKind::YtDlpDownload
                }
            );
            let final_path = folder.join(format!("Protected track (2).{ext}"));
            if cancel {
                assert!(cancelled_at.elapsed() < Duration::from_secs(6));
                assert_eq!(
                    disposition,
                    OutputDisposition::Cancelled,
                    "{name}: {result:?}"
                );
                assert_eq!(state.last_action, "Download stopped.");
                assert!(state.queue_items[0].last_error.is_none());
                assert!(!final_path.exists());
                let stages = fs::read_to_string(folder.join("stages.log")).unwrap();
                assert_eq!(
                    stages
                        .lines()
                        .filter(|s| *s != "controlled online producer")
                        .collect::<Vec<_>>(),
                    vec![stage]
                );
                for pid in &pids {
                    let output = Command::new("tasklist")
                        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
                        .output()
                        .unwrap();
                    assert!(
                        !String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()),
                        "live producer {pid}"
                    );
                }
                assert!(crate::infrastructure::tracked_test_pids().is_empty());
                let _cleanup = storage::exclusive_cleanup().unwrap();
            } else {
                assert_eq!(
                    disposition,
                    OutputDisposition::Published,
                    "{name}: {result:?}"
                );
                assert_eq!(
                    PathBuf::from(result.unwrap()),
                    final_path.canonicalize().unwrap()
                );
                assert!(final_path.is_file());
            }
            assert_eq!(fs::read(old).unwrap(), b"existing output remains intact");
            assert_eq!(
                crate::infrastructure::sha256_hex_file(source).unwrap(),
                source_hash
            );
            fs::write(folder.join("case-result.json"), serde_json::to_vec_pretty(&serde_json::json!({
                "stage":stage,"cached":cached,"cancel_requested":cancel,"source_kind":format!("{kind:?}"),
                "disposition":format!("{disposition:?}"),"pids":pids,"source_sha256":source_hash,
                "final_exists":final_path.exists(),"feedback":state.last_action
            })).unwrap()).unwrap();
            println!(
                "R-OUT-009 {stage} cached={cached} cancel={cancel}: actual dispatch, producer tree/pipes, unchanged source/old output and publication boundary verified"
            );
        }
    }
    chapter_shape_mismatch(root, tools, audio, server, &fixture);
}

fn chapter_shape_mismatch(
    root: &Path,
    tools: &ToolPaths,
    audio: &Path,
    server: &Server,
    fixture: &Path,
) {
    // A controlled producer ignores the valid probed chapters. It must fail
    // rather than publish an unreserved fallback after directory reservation.
    let mut fake = tools.clone();
    fake.yt_dlp = fixture.join("yt-dlp.exe").display().to_string();
    fake.ffmpeg = fixture.join("ffmpeg.exe").display().to_string();
    let folder = root.join("round3-chapter-shape-mismatch");
    let url = format!("{}/audio.wav?shape-mismatch", server.address);
    metadata(&fake, root, &url, "Mismatch", true);
    let mut state = new_state(root, &fake, "round3-chapter-shape-mismatch");
    let id = music_item(&mut state, &url, "Mismatch");
    fs::create_dir_all(&folder).unwrap();
    let old = folder.join("Mismatch.wav");
    fs::write(&old, b"unreserved fallback sentinel").unwrap();
    for (key, value) in [
        ("OUTPUT_POSTPASS_ROOT", folder.display().to_string()),
        ("OUTPUT_POSTPASS_BLOCK", "none".into()),
        ("OUTPUT_POSTPASS_MEDIA", audio.display().to_string()),
        ("OUTPUT_POSTPASS_MEDIA_EXT", "wav".into()),
    ] {
        unsafe {
            std::env::set_var(key, value);
        }
    }
    let mut selected = choice(MusicDownloadFormat::Wav);
    selected.save_mode = MusicDownloadSaveMode::SplitChapters;
    state.start_music_download_task_at(id, selected).unwrap();
    let (_, disposition, result, requested, split) = music_result(&mut state);
    assert_eq!(disposition, OutputDisposition::Failed);
    assert!(requested && !split);
    let error = result.unwrap_err();
    assert!(error.contains("no usable chapter files"), "{error}");
    assert_eq!(fs::read(old).unwrap(), b"unreserved fallback sentinel");
    assert!(!folder.join("Mismatch").exists());
    fs::write(folder.join("case-result.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "disposition":format!("{disposition:?}"),"error":error,"fixture":"controlled metadata/tool mismatch"
    })).unwrap()).unwrap();
}
