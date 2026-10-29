//! R-OUT-015: ordinary Stop retains ownership while exited-parent pipes drain.
use super::review_round1::{cached_stream, metadata, new_state};
use super::review_round3::{choice, music_item, music_result};
use super::*;

fn alive(pid: u32) -> bool {
    let result = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&result.stdout).contains(&pid.to_string())
}

fn log(folder: &Path) -> Vec<String> {
    fs::read_to_string(folder.join("stages.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn receipt(folder: &Path, data: serde_json::Value) {
    fs::write(
        folder.join("case-result.json"),
        serde_json::to_vec_pretty(&data).unwrap(),
    )
    .unwrap();
}

fn cleanup() -> storage::StorageUseGuard {
    let started = Instant::now();
    loop {
        match storage::exclusive_cleanup() {
            Ok(guard) => return guard,
            Err(error) => {
                assert!(started.elapsed() < Duration::from_secs(2), "{error}");
                thread::sleep(Duration::from_millis(25));
            }
        }
    }
}

pub(super) fn run(root: &Path, tools: &ToolPaths, audio: &Path, video: &Path, server: &Server) {
    let fixture = root.join("pipe-fixture.exe");
    let built = Command::new("rustc")
        .args(["--edition=2024", "--crate-name", "output_probe_fixture"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/app/state/output_probe_fixture.rs"))
        .arg("-o")
        .arg(&fixture)
        .output()
        .unwrap();
    fs::write(root.join("pipe-rustc.stdout.log"), &built.stdout).unwrap();
    fs::write(root.join("pipe-rustc.stderr.log"), &built.stderr).unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let probe = Path::new(&tools.ffmpeg)
        .parent()
        .unwrap()
        .join("ffprobe.exe");
    for route in ["ffprobe", "encoders", "postpass"] {
        for control in ["cancel", "natural", "failure"] {
            if control == "failure" && route != "postpass" {
                continue;
            }
            let name = format!("round5-{route}-{control}");
            let folder = root.join(&name);
            let bin = folder.join("tools");
            fs::create_dir_all(&bin).unwrap();
            for name in ["yt-dlp.exe", "ffmpeg.exe", "ffprobe.exe"] {
                fs::copy(&fixture, bin.join(name)).unwrap();
            }
            let mut fake = tools.clone();
            fake.yt_dlp = bin.join("yt-dlp.exe").display().to_string();
            fake.ffmpeg = bin.join("ffmpeg.exe").display().to_string();
            let source = if route == "encoders" { video } else { audio };
            let source_hash = crate::infrastructure::sha256_hex_file(source).unwrap();
            let url = format!("{}/audio.wav?{name}", server.address);
            let info = metadata(&fake, root, &url, "Orphan clip", route != "encoders");
            let mut state = new_state(root, &fake, &name);
            let id = music_item(&mut state, &url, "Orphan clip");
            let ext = match route {
                "ffprobe" => "flac",
                "encoders" => "mkv",
                _ => "mp3",
            };
            let old = folder.join(format!("Orphan clip.{ext}"));
            fs::write(&old, b"established final sentinel").unwrap();
            let final_path = folder.join(format!("Orphan clip (2).{ext}"));
            let local_input = folder.join("Orphan clip.mp4");
            let cache_input = if route == "ffprobe" {
                let seed = music_stream_seed_from_json(&info, &url).unwrap();
                let directory = cached_stream(&state, &seed, audio);
                state.restore_music_compact_cache_hit_if_available(id);
                Some(directory.join("audio.wav"))
            } else {
                None
            };
            if route == "encoders" {
                fs::copy(video, &local_input).unwrap();
            }
            for (key, value) in [
                ("OUTPUT_PROBE_ROOT", folder.display().to_string()),
                ("OUTPUT_PROBE_BLOCK", "none".into()),
                ("OUTPUT_PROBE_UNAVAILABLE", "none".into()),
                (
                    "OUTPUT_PROBE_ORPHAN_STAGE",
                    if route == "postpass" {
                        "convert".into()
                    } else {
                        route.into()
                    },
                ),
                ("OUTPUT_PROBE_EARLY_EXIT", control.into()),
                ("OUTPUT_PROBE_REAL_FFMPEG", tools.ffmpeg.clone()),
                ("OUTPUT_PROBE_REAL_FFPROBE", probe.display().to_string()),
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
            let started = Instant::now();
            if route == "encoders" {
                state.config.post_download_conversion_enabled = true;
                state.config.transcode_intent.container_policy =
                    crate::infrastructure::ContainerPolicy::Mkv;
                state.config.transcode_intent.video_codec_policy =
                    crate::infrastructure::VideoCodecPolicy::H264;
                assert!(state.maybe_start_builtin_transcode_post_process(
                    id,
                    &local_input.display().to_string()
                ));
            } else {
                state
                    .start_music_download_task_at(
                        id,
                        choice(if route == "ffprobe" {
                            MusicDownloadFormat::Flac
                        } else {
                            MusicDownloadFormat::Mp3
                        }),
                    )
                    .unwrap();
            }
            while !folder.join("active-pids.txt").exists() {
                assert!(
                    started.elapsed() < Duration::from_secs(15),
                    "{name}: no descendant, {:?}",
                    log(&folder)
                );
                thread::sleep(Duration::from_millis(10));
            }
            let pids: Vec<u32> = fs::read_to_string(folder.join("active-pids.txt"))
                .unwrap()
                .lines()
                .map(|p| p.parse().unwrap())
                .collect();
            let mut observed = false;
            if control != "natural" {
                while !crate::infrastructure::exited_tracked_test_pids().contains(&pids[0]) {
                    assert!(
                        started.elapsed() < Duration::from_secs(15),
                        "{name}: parent exit not observed"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
                assert!(!alive(pids[0]), "tracked parent still alive");
                assert!(
                    alive(pids[1]),
                    "inherited-pipe child must remain alive before Stop"
                );
                assert!(crate::infrastructure::tracked_test_pids().contains(&pids[0]));
                assert!(storage::exclusive_cleanup().is_err());
                assert!(state.active_workflows.values().all(|w| {
                    w.download_child
                        .as_ref()
                        .is_none_or(|h| h.lock().unwrap().is_none())
                }));
                observed = true;
                fs::write(folder.join("before-drain-release.json"),serde_json::to_vec_pretty(&serde_json::json!({
                    "parent_pid":pids[0],"parent_exit_observed_by_helper":true,"parent_alive":false,
                    "descendant_pid":pids[1],"descendant_alive":true,"cleanup_busy":true,
                    "workflow_main_child_empty":true
                })).unwrap()).unwrap();
                if control == "cancel" {
                    state.cancel_item_download(id);
                }
            }
            let stopping = Instant::now();
            if control != "natural" {
                // Hold the finite fixture open after Stop/parent failure. A
                // terminal event or ownership release here would be premature.
                thread::sleep(Duration::from_millis(200));
                assert!(alive(pids[1]));
                assert!(crate::infrastructure::tracked_test_pids().contains(&pids[0]));
                assert!(storage::exclusive_cleanup().is_err());
                assert!(!final_path.exists());
                assert!(state.active_workflows.values().any(|w| w.item_id == id));
                if control == "cancel" {
                    assert!(state.item_download_stopping(id));
                    assert_eq!(state.last_action, "Stopping download...");
                }
                while let Ok(event) = state.post_process_result_rx.try_recv() {
                    assert!(!matches!(event, PostProcessEvent::Finished(_)));
                }
                while let Ok(event) = state.music_download_event_rx.try_recv() {
                    assert!(!matches!(event, MusicDownloadEvent::Finished { .. }));
                }
                fs::write(folder.join("during-drain.json"), serde_json::to_vec_pretty(&serde_json::json!({
                    "descendant_alive":true,"tracked_guard_retained":true,"cleanup_busy":true,
                    "terminal_event_received":false,"final_exists":false,
                    "stop_requested":control == "cancel","item_stopping":state.item_download_stopping(id),
                    "active_workflow_retained":true,"held_ms":stopping.elapsed().as_millis()
                })).unwrap()).unwrap();
                fs::write(folder.join("finish-descendant.txt"), b"finish naturally").unwrap();
            }
            let (disposition, result) = if route == "encoders" {
                loop {
                    if let PostProcessEvent::Finished(result) = state
                        .post_process_result_rx
                        .recv_timeout(Duration::from_secs(12))
                        .unwrap()
                    {
                        break (result.disposition, result.result);
                    }
                }
            } else {
                let (_, disposition, result, _, _) = music_result(&mut state);
                (disposition, result)
            };
            let elapsed = stopping.elapsed().as_millis();
            let expected = match control {
                "cancel" => OutputDisposition::Cancelled,
                "failure" => OutputDisposition::Failed,
                _ => OutputDisposition::Published,
            };
            assert_eq!(disposition, expected, "{name}: {result:?}");
            if control != "natural" {
                assert!(!final_path.exists());
                assert!(result.is_err());
                assert_eq!(
                    fs::read(folder.join("descendant-finished.txt")).unwrap(),
                    b"natural completion"
                );
                let stages = log(&folder);
                assert_eq!(
                    stages.last().unwrap(),
                    if route == "postpass" {
                        "convert"
                    } else {
                        route
                    }
                );
                assert_eq!(
                    stages.iter().filter(|s| s.as_str() == "convert").count(),
                    usize::from(route == "postpass")
                );
            } else {
                assert_eq!(
                    PathBuf::from(result.unwrap()),
                    final_path.canonicalize().unwrap()
                );
                assert!(final_path.is_file());
            }
            for pid in &pids {
                assert!(!alive(*pid), "producer {pid} still alive");
            }
            assert!(crate::infrastructure::tracked_test_pids().is_empty());
            assert!(crate::infrastructure::exited_tracked_test_pids().is_empty());
            let _cleanup = cleanup();
            assert_eq!(fs::read(&old).unwrap(), b"established final sentinel");
            assert_eq!(
                crate::infrastructure::sha256_hex_file(source).unwrap(),
                source_hash
            );
            if route == "encoders" {
                assert_eq!(
                    crate::infrastructure::sha256_hex_file(&local_input).unwrap(),
                    source_hash
                );
            }
            if let Some(path) = cache_input.as_ref() {
                assert_eq!(
                    crate::infrastructure::sha256_hex_file(path).unwrap(),
                    source_hash
                );
            }
            receipt(
                &folder,
                serde_json::json!({"route":route,"control":control,"pids":pids,"parent_exit_observed_before_release":observed,
                "disposition":format!("{disposition:?}"),"terminal_wait_ms":elapsed,"source_sha256":source_hash,
                "cache_input":cache_input,"final_exists":final_path.exists(),"stages":log(&folder)}),
            );
            println!(
                "R-OUT-015 {route} {control}: natural pipe drain retains ownership and protects publication"
            );
        }
    }
    let folder = root.join("round5-pipe-capture");
    fs::create_dir_all(&folder).unwrap();
    for (key, value) in [
        ("OUTPUT_PROBE_ROOT", folder.display().to_string()),
        ("OUTPUT_PROBE_ORPHAN_STAGE", "capture".into()),
        ("OUTPUT_PROBE_EARLY_EXIT", "natural".into()),
    ] {
        unsafe {
            std::env::set_var(key, value);
        }
    }
    let mut command = Command::new(&fixture);
    command.arg("--verify-pipe-capture");
    let output = crate::infrastructure::run_tracked_command_output_cancellable(
        &mut command,
        "pipe capture control",
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(output.status.success());
    assert_eq!(&output.stdout[..64 * 4096], &vec![b'o'; 64 * 4096]);
    assert_eq!(&output.stdout[64 * 4096..], b"stdout tail drained");
    assert_eq!(&output.stderr[..64 * 4096], &vec![b'e'; 64 * 4096]);
    assert_eq!(&output.stderr[64 * 4096..], b"stderr tail drained");
    fs::write(folder.join("stdout.bin"), &output.stdout).unwrap();
    fs::write(folder.join("stderr.bin"), &output.stderr).unwrap();
    let _cleanup = cleanup();
    receipt(
        &folder,
        serde_json::json!({"control":"natural drain","stdout_bytes":output.stdout.len(),"stderr_bytes":output.stderr.len(),"status":output.status.code()}),
    );
    drop(_cleanup);
    let folder = root.join("round5-spawn-failure");
    fs::create_dir_all(&folder).unwrap();
    let mut command = Command::new(folder.join("missing.exe"));
    assert!(
        crate::infrastructure::run_tracked_command_output_cancellable(
            &mut command,
            "spawn failure control",
            &AtomicBool::new(false)
        )
        .is_err()
    );
    let _cleanup = cleanup();
    assert!(crate::infrastructure::tracked_test_pids().is_empty());
    receipt(
        &folder,
        serde_json::json!({"control":"spawn failure","cleanup_available":true}),
    );
    println!("R-OUT-015 acceptance passed: {}", root.display());
}
