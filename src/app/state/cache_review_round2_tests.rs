//! Actual export and streaming-owner regressions for R-CACHE-004/005.
use super::*;
use crate::infrastructure::storage_paths as storage;

#[test]
#[ignore = "Requires local Python/yt-dlp packages, rustc, and FFmpeg; no external services"]
fn review_round2_exports_and_import_owners() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/cache-location-validation")
        .join(format!("review-round2-{}", std::process::id()));
    fs::create_dir_all(root.join("app")).unwrap();
    fs::create_dir_all(root.join("system-temp")).unwrap();
    for phase in [
        "portable",
        "temporary",
        "restart",
        "startup-portable",
        "startup-temporary",
    ] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "app::state::cache_review_round2_tests::round2_child",
                "--nocapture",
            ])
            .env("CACHE_REVIEW_ROUND2_ROOT", &root)
            .env("CACHE_REVIEW_ROUND2_PHASE", phase)
            .output()
            .unwrap();
        println!("{}", String::from_utf8_lossy(&output.stdout));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    println!("Round 2 actual export/import fixture: {}", root.display());
}

#[test]
fn round2_child() {
    let Some(root) = std::env::var_os("CACHE_REVIEW_ROUND2_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let phase = std::env::var("CACHE_REVIEW_ROUND2_PHASE").unwrap();
    let app = root.join("app");
    let temp = root.join("system-temp");
    if phase.starts_with("startup-") {
        let output = short_path(&app.join("cache")).join("audio/restored-output/new-child");
        storage::initialize_output_fixture(
            &app,
            &temp,
            if phase == "startup-portable" {
                CacheLocationMode::V2Cache
            } else {
                CacheLocationMode::WindowsTemp
            },
            "cache",
            &output,
        );
        assert!(
            storage::error()
                .unwrap()
                .contains("outside application cache")
        );
        assert!(!app.join("cache/audio/restored-output").exists());
        println!("R-CACHE-004 ({phase}): restored aliased output blocked during actual startup");
        return;
    }
    storage::initialize_fixture(
        &app,
        &temp,
        if phase == "portable" {
            CacheLocationMode::V2Cache
        } else {
            CacheLocationMode::WindowsTemp
        },
        "cache",
    );
    storage::require_ready().unwrap();
    if phase == "restart" {
        for mode in ["portable", "temporary"] {
            verify_exports(&app.join(format!("final exports 空间/{mode}")));
        }
        println!("R-CACHE-004 actual copied/converted files and neighbors survive restart");
        return;
    }
    let mut tools = ToolPaths {
        ffmpeg: std::env::var("YT_DLP_GUI_TEST_FFMPEG").expect("Set real FFmpeg path"),
        ..Default::default()
    };
    let config = AppConfig::default().with_fixture_file(app.join(format!("{phase}-settings.yaml")));
    let mut state = AppState::from_runtime(config, tools.clone());
    export_boundaries(&app, &phase, &tools, &mut state);
    tools.yt_dlp = build_import_tool(&app, &temp).display().to_string();
    state.tool_paths = tools.clone();
    for owner in [
        "batch",
        "batch-cancel",
        "flat-update",
        "flat-list",
        "batch-drain",
        "flat-list-drain",
        "flat-update-drain",
    ] {
        import_owner(&app, &phase, owner, &tools, &mut state);
    }
}

fn choice(convert: bool) -> MusicDownloadChoice {
    MusicDownloadChoice {
        mode: if convert {
            MusicDownloadMode::Unified
        } else {
            MusicDownloadMode::Original
        },
        unified_format: MusicDownloadFormat::Flac,
        embed_cover: false,
        write_tags: false,
        ..Default::default()
    }
}

fn job(source: &Path, output: &Path, convert: bool) -> MusicDownloadJob {
    MusicDownloadJob {
        output: Default::default(),
        item_id: 1,
        workflow_id: 1,
        source_url: "reviewcache:offline".into(),
        title: if convert { "converted" } else { "copied" }.into(),
        album_title: String::new(),
        output_dir: output.to_owned(),
        choice: choice(convert),
        source_acodec: "pcm_s16le".into(),
        cache_media_path: Some(source.to_owned()),
        cover_path: None,
        cover_cache_dir: None,
        thumbnail_url: String::new(),
        use_cookies: false,
    }
}

fn export(
    source: &Path,
    output: &Path,
    convert: bool,
    tools: &ToolPaths,
) -> (MusicDownloadSourceKind, Result<String, String>) {
    let (tx, rx) = mpsc::channel();
    // This is the production worker, including source-branch selection and its
    // own storage lease. The assertion does not borrow an artificial test lease.
    run_music_download_worker(
        tools.clone(),
        job(source, output, convert),
        tx,
        Arc::new(Mutex::new(None)),
        Arc::new(AtomicBool::new(false)),
    );
    rx.into_iter()
        .find_map(|event| match event {
            MusicDownloadEvent::Finished {
                source_kind,
                result,
                ..
            } => Some((source_kind, result)),
            _ => None,
        })
        .unwrap()
}

#[cfg(windows)]
fn short_path(path: &Path) -> PathBuf {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetShortPathNameW(long: *const u16, short: *mut u16, capacity: u32) -> u32;
    }
    let long: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut short = vec![0u16; 32768];
    let count = unsafe { GetShortPathNameW(long.as_ptr(), short.as_mut_ptr(), short.len() as u32) };
    assert!(count > 0 && count < short.len() as u32);
    PathBuf::from(std::ffi::OsString::from_wide(&short[..count as usize]))
}

#[cfg(not(windows))]
fn short_path(path: &Path) -> PathBuf {
    path.to_owned()
}

fn export_boundaries(app: &Path, phase: &str, tools: &ToolPaths, state: &mut AppState) {
    let source = storage::cache_directory("audio")
        .unwrap()
        .join("source.wav");
    let generated = Command::new(&tools.ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.1",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let paths = storage::paths();
    for (index, root) in [
        &paths.cache_root,
        &paths.portable_root,
        &paths.temporary_root,
    ]
    .iter()
    .enumerate()
    {
        // Create only the managed parent, including inactive roots. Export
        // descendants remain absent, exercising nearest-existing resolution.
        fs::create_dir_all(root.join("audio")).unwrap();
        let mut aliases = vec![root.to_path_buf()];
        #[cfg(windows)]
        {
            let short = short_path(root);
            assert_ne!(
                short.to_string_lossy().to_lowercase(),
                root.to_string_lossy().to_lowercase(),
                "Fixture must exercise a genuine short-name alias"
            );
            aliases.push(short.clone());
            aliases.push(PathBuf::from(format!("{}.", root.display())));
            aliases.push(PathBuf::from(format!("{} ", root.display())));
            aliases.push(PathBuf::from(format!(r"\\?\{}", short.display())));
            aliases.push(root.canonicalize().unwrap());
        }
        for (alias_index, alias) in aliases.iter().enumerate() {
            for existing in [false, true] {
                let candidate = if existing {
                    alias.join("audio")
                } else {
                    alias.join(format!("audio/rejected-{index}-{alias_index}/new-child"))
                };
                assert!(
                    storage::validate_output_destination(&candidate).is_err(),
                    "{}",
                    candidate.display()
                );
                // Exercise the real per-item audio scheduler as well as a
                // directly restored worker job. No metadata/network is needed.
                let previous_tools = state.tool_paths.clone();
                state.tool_paths.yt_dlp = tools.ffmpeg.clone();
                let mut item = QueueItem::new(1, "reviewcache:offline", "Restored cached track");
                item.selection.output_dir = candidate.display().to_string();
                state.queue_items = vec![item];
                for convert in [false, true] {
                    assert!(
                        state
                            .start_music_download_task_at(1, choice(convert))
                            .is_err()
                    );
                    assert!(!state.has_running_download_workflow());
                }
                state.queue_items.clear();
                state.tool_paths = previous_tools;
                for convert in [false, true] {
                    let (branch, result) = export(&source, &candidate, convert, tools);
                    assert_eq!(
                        branch,
                        if convert {
                            MusicDownloadSourceKind::CacheConvert
                        } else {
                            MusicDownloadSourceKind::CacheCopy
                        }
                    );
                    assert!(
                        result.is_err(),
                        "Unsafe export succeeded: {}",
                        candidate.display()
                    );
                }
                assert!(
                    !root
                        .join(format!("audio/rejected-{index}-{alias_index}"))
                        .exists()
                );
                assert!(!root.join("audio/copied.wav").exists());
                assert!(!root.join("audio/converted.flac").exists());
            }
        }
    }
    let final_dir = app.join(format!("final exports 空间/{phase}"));
    for convert in [false, true] {
        let (_, result) = export(&source, &final_dir, convert, tools);
        let output = PathBuf::from(result.unwrap());
        let bytes = fs::read(&output).unwrap();
        if convert {
            assert!(bytes.starts_with(b"fLaC"));
        } else {
            assert_eq!(bytes, fs::read(&source).unwrap());
        }
        fs::write(output.with_extension("expected"), &bytes).unwrap();
    }
    fs::write(final_dir.join("notes.txt"), b"unknown user-owned neighbor").unwrap();
    state.clear_music_stream_cache();
    state.clear_app_cache();
    assert!(!source.exists());
    verify_exports(&final_dir);
    println!(
        "R-CACHE-004 ({phase}): actual CacheCopy/FFmpeg CacheConvert reject aliases for active/inactive roots before writes; valid exports survive audio/all cleanup"
    );
}

fn verify_exports(root: &Path) {
    for name in ["copied.wav", "converted.flac"] {
        let path = root.join(name);
        assert_eq!(
            fs::read(&path).unwrap(),
            fs::read(path.with_extension("expected")).unwrap()
        );
    }
    assert_eq!(
        fs::read(root.join("notes.txt")).unwrap(),
        b"unknown user-owned neighbor"
    );
}

fn build_import_tool(app: &Path, temp: &Path) -> PathBuf {
    let python = std::env::var("YT_DLP_GUI_TEST_PYTHON").unwrap();
    let packages = std::env::var("YT_DLP_GUI_GALLERY_TEST_PACKAGES").unwrap();
    let script = app.join("stream-import.py");
    fs::write(&script, include_str!("cache_review_stream_fixture.py")).unwrap();
    let launcher = app.join("stream-import-launcher.rs");
    fs::write(
        &launcher,
        format!(
            r#"fn main() {{
        let status = std::process::Command::new({python:?})
            .arg("-B").arg({script:?}).args(std::env::args_os().skip(1))
            .env("PYTHONPATH", {packages:?}).env("TEMP", {temp:?}).env("TMP", {temp:?})
            .status().unwrap();
        std::process::exit(status.code().unwrap_or(1));
    }}"#,
            script = script.display().to_string(),
            temp = temp.display().to_string()
        ),
    )
    .unwrap();
    let exe = app.join("stream-import.exe");
    let result = Command::new("rustc")
        .args(["--edition=2024", "--crate-name", "review_import_fixture"])
        .arg(&launcher)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    exe
}

fn wait_file(path: &Path) {
    let end = Instant::now() + Duration::from_secs(35);
    while !path.exists() {
        assert!(Instant::now() < end, "Timed out: {}", path.display());
        thread::sleep(Duration::from_millis(20));
    }
}

fn busy(cache_file: Option<&Path>, label: &str) {
    let error = storage::exclusive_cleanup()
        .err()
        .expect("Actual streaming owner must block cleanup");
    assert!(error.to_lowercase().contains("in use"), "{label}: {error}");
    if let Some(file) = cache_file {
        assert!(file.is_file(), "Active cache was deleted: {label}");
    }
}

fn import_owner(app: &Path, phase: &str, owner: &str, tools: &ToolPaths, state: &mut AppState) {
    let control = app.join(format!("import-{phase}-{owner}"));
    fs::create_dir_all(&control).unwrap();
    let source = format!("reviewstream:{}", control.display());
    let cancel = Arc::new(AtomicBool::new(false));
    let child = Arc::new(Mutex::new(None));
    let (done_tx, done_rx) = mpsc::channel();
    if owner.starts_with("batch") {
        let (tx, rx) = mpsc::channel();
        let tools = tools.clone();
        let source = source.clone();
        let cancel = cancel.clone();
        let child = child.clone();
        thread::spawn(move || {
            run_batch_add_worker(
                tools,
                source,
                false,
                None,
                "Untitled".into(),
                "Imported {tail}".into(),
                false,
                1,
                tx,
                child,
                cancel,
            );
            let events: Vec<_> = rx.into_iter().collect();
            let success = events.iter().any(|e| {
                matches!(
                    e,
                    BatchAddEvent::Finished { .. } | BatchAddEvent::Cancelled { .. }
                )
            });
            done_tx.send(success).unwrap();
        });
    } else if owner.starts_with("flat-list") {
        let tools = tools.clone();
        thread::spawn(move || {
            done_tx
                .send(
                    flat_music_entries_from_url(
                        tools,
                        &source,
                        false,
                        "Untitled",
                        "Imported {tail}",
                    )
                    .is_ok(),
                )
                .unwrap();
        });
    } else {
        state.spawn_music_flat_update_worker(1, source, false);
        drop(done_tx);
    }
    wait_file(&control.join("startup"));
    busy(None, "startup");
    fs::write(control.join("start"), b"go").unwrap();
    wait_file(&control.join("cache-access"));
    let cache_file = storage::paths()
        .cache_root
        .join("yt-dlp/review-stream/active.json");
    busy(Some(&cache_file), "actual engine cache access");
    if owner == "batch-cancel" {
        cancel.store(true, Ordering::Relaxed);
        // Setting cancellation alone does not release storage ownership.
        busy(
            Some(&cache_file),
            "cancellation requested before process teardown",
        );
        request_batch_add_stop(&child);
    } else {
        fs::write(control.join("finish"), b"go").unwrap();
        if owner.ends_with("drain") {
            wait_file(&control.join("draining"));
            busy(Some(&cache_file), "parent exited; output pipe still owned");
            fs::write(control.join("drain-release"), b"go").unwrap();
        }
    }
    if owner.starts_with("flat-update") {
        let end = Instant::now() + Duration::from_secs(35);
        loop {
            assert!(Instant::now() < end, "flat update completion timeout");
            match state
                .music_stream_result_rx
                .recv_timeout(Duration::from_millis(100))
            {
                Ok(MusicStreamResolveEvent::FlatUpdate { result, .. }) => {
                    assert_eq!(result.is_ok(), !owner.ends_with("drain"));
                    break;
                }
                _ => {}
            }
        }
    } else {
        assert!(done_rx.recv_timeout(Duration::from_secs(35)).unwrap());
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match storage::exclusive_cleanup() {
            Ok(_cleanup) => {
                storage::clear_disposable(storage::paths(), &[]).unwrap();
                break;
            }
            Err(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            Err(e) => panic!("Lease not released: {e}"),
        }
    }
    assert!(!cache_file.exists());
    thread::sleep(Duration::from_millis(150));
    assert!(
        !cache_file.exists(),
        "Cache writer survived completed cleanup"
    );
    fs::write(control.join("assertions.json"), serde_json::json!({"owner":owner,"mode":phase,"busy_startup":true,"busy_cache_access":true,"cancel":owner=="batch-cancel","output_teardown":owner.ends_with("drain"),"released":true,"no_late_cache_write":true}).to_string()).unwrap();
    println!(
        "R-CACHE-005 ({phase}/{owner}): busy startup/cache/cancel-or-drain; eventual release and no writes after cleanup"
    );
}
