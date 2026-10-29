//! Round 4: effective external formats and operation-owned conversion probes.
use super::review_round1::{cached_stream, finished, metadata, new_state};
use super::review_round3::{choice, music_item, music_result};
use super::*;

pub(super) fn run(root: &Path, tools: &ToolPaths, audio: &Path, video: &Path, server: &Server) {
    external_formats(root, tools, audio, server);
    conversion_probes(root, tools, audio, video, server);
    println!("R-OUT-008/010 acceptance passed: {}", root.display());
}

fn receipt(folder: &Path, value: serde_json::Value) {
    fs::write(
        folder.join("case-result.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
}

fn external_formats(root: &Path, tools: &ToolPaths, audio: &Path, server: &Server) {
    for (ext, codec) in [("m4a", "aac"), ("webm", "libopus")] {
        let converted = Command::new(&tools.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
            .arg(audio)
            .args(["-c:a", codec])
            .arg(root.join(format!("dual-audio.{ext}")))
            .output()
            .unwrap();
        assert!(
            converted.status.success(),
            "{}",
            String::from_utf8_lossy(&converted.stderr)
        );
    }
    for chapters in [false, true] {
        for case in [
            "occupied-skip",
            "wrong-skip",
            "wrong-keep",
            "occupied-keep",
            "absent",
            "chapters",
        ] {
            if case == "chapters" && !chapters {
                continue;
            }
            let name = format!("round4-format-{chapters}-{case}");
            let url = format!("{}/dual-source.wav?{name}", server.address);
            let mut info = metadata(tools, root, &url, "Track", true);
            if case != "chapters" {
                info.as_object_mut().unwrap().remove("chapters");
            }
            for key in ["ext", "url", "format_id", "acodec", "vcodec", "protocol"] {
                info.as_object_mut().unwrap().remove(key);
            }
            info["formats"] = serde_json::json!([
                {"format_id":"external-aac","url":format!("{}/dual-audio.m4a",server.address),"ext":"m4a","acodec":"aac","vcodec":"none","protocol":"http","abr":96,"tbr":96,"quality":0},
                {"format_id":"app-opus","url":format!("{}/dual-audio.webm",server.address),"ext":"webm","acodec":"opus","vcodec":"none","protocol":"http","abr":192,"tbr":192,"quality":10}
            ]);
            fs::write(
                root.join("round1-input.json"),
                serde_json::to_vec(&info).unwrap(),
            )
            .unwrap();
            let mut configured = tools.clone();
            let config = root.join("external-format.conf");
            fs::write(
                &config,
                format!(
                    "--load-info-json \"{}\"\n--format external-aac\n",
                    root.join("round1-input.json").display()
                ),
            )
            .unwrap();
            configured.yt_dlp_config = config.display().to_string();
            let cancel = AtomicBool::new(false);
            assert_eq!(
                tools
                    .music_output_extension(&url, "bestaudio/best", false, &cancel)
                    .unwrap(),
                "webm"
            );
            assert_eq!(
                configured
                    .music_output_extension(&url, "bestaudio/best", false, &cancel)
                    .unwrap(),
                "m4a"
            );
            let mut state = new_state(root, &configured, &name);
            let id = music_item(&mut state, &url, "Track");
            let folder = root.join(&name);
            fs::create_dir_all(&folder).unwrap();
            let wrong = folder.join("Track.webm");
            let target = folder.join("Track.m4a");
            fs::write(&wrong, b"unrelated app-selected container").unwrap();
            let occupied = case.starts_with("occupied") || case == "chapters";
            if occupied {
                fs::write(&target, b"existing actual selected container").unwrap();
            }
            state.config.output_existing_file = if case.ends_with("keep") {
                ExistingFilePolicy::KeepBoth
            } else {
                ExistingFilePolicy::Skip
            };
            let mut selected = choice(MusicDownloadFormat::Mp3);
            selected.mode = MusicDownloadMode::Original;
            if chapters {
                selected.save_mode = MusicDownloadSaveMode::SplitChapters;
            }
            let before = server.media_get_count();
            state.start_music_download_task_at(id, selected).unwrap();
            let (_, disposition, result, requested, split) = music_result(&mut state);
            let path = PathBuf::from(result.unwrap());
            let skip = case == "occupied-skip";
            assert_eq!(
                disposition,
                if skip {
                    OutputDisposition::SkippedExisting
                } else {
                    OutputDisposition::Published
                }
            );
            assert_eq!(requested, chapters);
            assert_eq!(split, case == "chapters");
            if case == "chapters" {
                assert_eq!(path.file_name().unwrap(), "Track");
                let files: Vec<_> = fs::read_dir(&path)
                    .unwrap()
                    .map(|p| p.unwrap().path())
                    .collect();
                assert_eq!(files.len(), 2);
                assert!(files.iter().all(|p| p.extension().unwrap() == "m4a"));
            } else {
                assert_eq!(
                    path.file_name().unwrap(),
                    if case == "occupied-keep" {
                        "Track (2).m4a"
                    } else {
                        "Track.m4a"
                    }
                );
            }
            assert_eq!(
                fs::read(&wrong).unwrap(),
                b"unrelated app-selected container"
            );
            if occupied {
                assert_eq!(
                    fs::read(&target).unwrap(),
                    b"existing actual selected container"
                );
            }
            if skip {
                assert_eq!(server.media_get_count(), before);
            } else {
                assert!(server.media_get_count() > before);
            }
            receipt(
                &folder,
                serde_json::json!({"case":case,"chapters_requested":requested,"chapters_split":split,
                "app_extension":"webm","external_extension":"m4a","path":path,"disposition":format!("{disposition:?}"),
                "media_gets_before":before,"media_gets_after":server.media_get_count()}),
            );
        }
    }
    println!(
        "R-OUT-008: external M4A versus app WebM selection, full/no-chapters/chapter Original dispatch and pre-transfer Skip passed"
    );
}

fn stages(folder: &Path) -> Vec<String> {
    fs::read_to_string(folder.join("stages.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn conversion_probes(root: &Path, tools: &ToolPaths, audio: &Path, video: &Path, server: &Server) {
    let executable = root.join("probe-fixture.exe");
    let compiled = Command::new("rustc")
        .args(["--edition=2024", "--crate-name", "output_probe_fixture"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app/state/output_probe_fixture.rs"))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    fs::write(root.join("probe-rustc.stdout.log"), &compiled.stdout).unwrap();
    fs::write(root.join("probe-rustc.stderr.log"), &compiled.stderr).unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let real_probe = Path::new(&tools.ffmpeg)
        .parent()
        .unwrap()
        .join("ffprobe.exe");
    assert!(real_probe.is_file());
    for route in [
        "music-cache",
        "music-online",
        "standalone-probe",
        "builtin-probe",
        "standalone-encoders",
        "builtin-encoders",
    ] {
        for control in ["cancel", "success", "unavailable"] {
            let name = format!("round4-probe-{route}-{control}");
            let folder = root.join(&name);
            let tool_folder = folder.join("tools");
            fs::create_dir_all(&tool_folder).unwrap();
            let blocked = if route.ends_with("encoders") {
                "encoders"
            } else {
                "ffprobe"
            };
            for tool in ["ffmpeg.exe", "yt-dlp.exe", "ffprobe.exe"] {
                if tool == "ffprobe.exe" && blocked == "ffprobe" && control == "unavailable" {
                    continue;
                }
                fs::copy(&executable, tool_folder.join(tool)).unwrap();
            }
            let mut fake = tools.clone();
            fake.yt_dlp = tool_folder.join("yt-dlp.exe").display().to_string();
            fake.ffmpeg = tool_folder.join("ffmpeg.exe").display().to_string();
            let source = if route.starts_with("music") {
                audio
            } else {
                video
            };
            let source_hash = crate::infrastructure::sha256_hex_file(source).unwrap();
            let url = format!(
                "{}/probe-source.{}?{name}",
                server.address,
                if route.starts_with("music") {
                    "wav"
                } else {
                    "mp4"
                }
            );
            let info = metadata(&fake, root, &url, "Probe clip", route.starts_with("music"));
            let mut state = new_state(root, &fake, &name);
            let id = music_item(&mut state, &url, "Probe clip");
            let old_ext = if route == "music-cache" {
                "flac"
            } else if route == "music-online" {
                "mp3"
            } else {
                "mkv"
            };
            let old = folder.join(format!("Probe clip.{old_ext}"));
            fs::write(&old, b"existing final output").unwrap();
            let final_path = folder.join(format!("Probe clip (2).{old_ext}"));
            let staged_source = folder.join("Probe clip.mp4");
            if route.starts_with("standalone") {
                fs::copy(video, &staged_source).unwrap();
            }
            if route == "music-cache" {
                let seed = music_stream_seed_from_json(&info, &url).unwrap();
                cached_stream(&state, &seed, audio);
                state.restore_music_compact_cache_hit_if_available(id);
            }
            for (key, value) in [
                ("OUTPUT_PROBE_ROOT", folder.display().to_string()),
                (
                    "OUTPUT_PROBE_BLOCK",
                    if control == "cancel" {
                        blocked.into()
                    } else {
                        "none".into()
                    },
                ),
                (
                    "OUTPUT_PROBE_UNAVAILABLE",
                    if control == "unavailable" {
                        blocked.into()
                    } else {
                        "none".into()
                    },
                ),
                ("OUTPUT_PROBE_REAL_FFMPEG", tools.ffmpeg.clone()),
                (
                    "OUTPUT_PROBE_REAL_FFPROBE",
                    real_probe.display().to_string(),
                ),
                ("OUTPUT_PROBE_REAL_YTDLP", tools.yt_dlp.clone()),
                ("OUTPUT_PROBE_MEDIA", source.display().to_string()),
                (
                    "OUTPUT_PROBE_CACHE",
                    storage::paths()
                        .cache_root
                        .canonicalize()
                        .unwrap()
                        .display()
                        .to_string(),
                ),
            ] {
                unsafe {
                    std::env::set_var(key, value);
                }
            }
            if route.starts_with("music") {
                let selected = choice(if route == "music-cache" {
                    MusicDownloadFormat::Flac
                } else {
                    MusicDownloadFormat::Mp3
                });
                state.start_music_download_task_at(id, selected).unwrap();
            } else {
                state.config.post_download_conversion_enabled = true;
                state.config.transcode_intent.container_policy =
                    crate::infrastructure::ContainerPolicy::Mkv;
                if blocked == "encoders" {
                    state.config.transcode_intent.video_codec_policy =
                        crate::infrastructure::VideoCodecPolicy::H264;
                }
                if route.starts_with("standalone") {
                    assert!(state.maybe_start_builtin_transcode_post_process(
                        id,
                        &staged_source.display().to_string()
                    ));
                } else {
                    state.queue_items[0].metadata_state = MetadataState::Idle;
                    state.queue_items[0].selection.file_name = "Probe clip".into();
                    state.start_download_task_at(id, true).unwrap();
                }
            }
            let mut pids = Vec::new();
            if control == "cancel" {
                let waiting = Instant::now();
                while !folder.join("active-pids.txt").exists() {
                    assert!(
                        waiting.elapsed() < Duration::from_secs(15),
                        "{name}: probe not active; {:?}",
                        stages(&folder)
                    );
                    thread::sleep(Duration::from_millis(25));
                }
                pids = fs::read_to_string(folder.join("active-pids.txt"))
                    .unwrap()
                    .lines()
                    .map(|p| p.parse::<u32>().unwrap())
                    .collect();
                assert!(storage::exclusive_cleanup().is_err());
                assert!(state.active_workflows.values().all(|w| {
                    w.download_child
                        .as_ref()
                        .is_none_or(|h| h.lock().unwrap().is_none())
                }));
                state.cancel_item_download(id);
            }
            let stopped = Instant::now();
            let (disposition, result) = if route.starts_with("music") {
                let (_, disposition, result, _, _) = music_result(&mut state);
                (disposition, result)
            } else if route.starts_with("standalone") {
                loop {
                    let event = state
                        .post_process_result_rx
                        .recv_timeout(Duration::from_secs(40))
                        .unwrap();
                    if let PostProcessEvent::Finished(result) = event {
                        break (result.disposition, result.result);
                    }
                }
            } else {
                let result = finished(&state);
                (result.disposition, result.result)
            };
            if control == "cancel" {
                assert!(stopped.elapsed() < Duration::from_secs(6), "{name}");
                assert_eq!(
                    disposition,
                    OutputDisposition::Cancelled,
                    "{name}: {result:?}"
                );
                assert!(result.unwrap_err().to_lowercase().contains("cancel"));
                assert!(!final_path.exists());
                let seen = stages(&folder);
                assert_eq!(seen.last().unwrap(), blocked);
                assert!(!seen.iter().any(|s| s == "convert"));
                for pid in &pids {
                    let output = Command::new("tasklist")
                        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
                        .output()
                        .unwrap();
                    assert!(!String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()));
                }
                assert!(crate::infrastructure::tracked_test_pids().is_empty());
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
                assert!(stages(&folder).iter().any(|s| s == "convert"));
            }
            // Finished is sent just before the dispatch thread drops its owner.
            // Wait for that real release, with a strict deadline, rather than
            // racing the final send against a same-instant cleanup attempt.
            let release_started = Instant::now();
            let _cleanup = loop {
                match storage::exclusive_cleanup() {
                    Ok(guard) => break guard,
                    Err(error) => {
                        assert!(
                            release_started.elapsed() < Duration::from_secs(2),
                            "{name}: {error}"
                        );
                        thread::sleep(Duration::from_millis(25));
                    }
                }
            };
            assert_eq!(fs::read(&old).unwrap(), b"existing final output");
            assert_eq!(
                crate::infrastructure::sha256_hex_file(source).unwrap(),
                source_hash
            );
            if route.starts_with("standalone") {
                assert_eq!(
                    crate::infrastructure::sha256_hex_file(&staged_source).unwrap(),
                    source_hash
                );
            }
            receipt(
                &folder,
                serde_json::json!({"route":route,"control":control,"probe":blocked,"pids":pids,
                "disposition":format!("{disposition:?}"),"stages":stages(&folder),"final_exists":final_path.exists(),"source_sha256":source_hash,
                "cleanup_release_ms":release_started.elapsed().as_millis()}),
            );
            println!(
                "R-OUT-010 {route} {control}: actual workflow, producer/readers, preserved files and cleanup verified"
            );
        }
    }
}
