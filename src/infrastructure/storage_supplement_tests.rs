//! Isolated lifetime tests and opt-in real gallery-dl filesystem checks.
use super::{fixture, run_storage_child, write};
use crate::infrastructure::{
    browser_native_cookies::NativeCookies,
    gallery_dl::{GalleryDlBackend, GalleryDlRuntime},
    portable_cleanup, storage_paths,
    tools::{CacheLocationMode, ToolPaths},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

fn child(app: &Path, temp: &Path, mode: &str, real: bool) -> std::process::Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "infrastructure::storage_paths::tests::supplement::storage_supplement_child",
            "--nocapture",
        ])
        .env("YT_DLP_GUI_SUPPLEMENT_APP", app)
        .env("YT_DLP_GUI_SUPPLEMENT_TEMP", temp)
        .env("YT_DLP_GUI_SUPPLEMENT_MODE", mode)
        .env("YT_DLP_GUI_SUPPLEMENT_REAL", if real { "1" } else { "0" })
        .output()
        .unwrap()
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
}

#[test]
fn storage_supplement_snapshot_and_gallery_lifetimes() {
    let (app, temp) = fixture("supplement");
    for mode in ["portable", "temporary"] {
        assert_success(&child(&app, &temp, mode, false));
    }
}

#[test]
#[ignore = "Set YT_DLP_GUI_TEST_PYTHON and YT_DLP_GUI_GALLERY_TEST_PACKAGES; localhost only"]
fn storage_supplement_real_gallery() {
    let (app, temp) = fixture("real-gallery");
    for mode in ["portable", "temporary"] {
        let output = child(&app, &temp, mode, true);
        println!("{}", String::from_utf8_lossy(&output.stdout));
        assert_success(&output);
    }
    println!("Gallery fixture: {}", app.parent().unwrap().display());
}

fn stale(path: &Path) {
    let mut options = fs::OpenOptions::new();
    options.write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x02000000); // FILE_FLAG_BACKUP_SEMANTICS for fixture directories.
    }
    options
        .open(path)
        .unwrap()
        .set_times(
            fs::FileTimes::new()
                .set_modified(SystemTime::now() - Duration::from_secs(25 * 60 * 60)),
        )
        .unwrap();
}

#[test]
fn storage_supplement_child() {
    let Some(app) = std::env::var_os("YT_DLP_GUI_SUPPLEMENT_APP") else {
        return;
    };
    let app = PathBuf::from(app);
    let temp = PathBuf::from(std::env::var_os("YT_DLP_GUI_SUPPLEMENT_TEMP").unwrap());
    let mode = std::env::var("YT_DLP_GUI_SUPPLEMENT_MODE").unwrap();
    let mode = if mode == "portable" {
        CacheLocationMode::V2Cache
    } else {
        CacheLocationMode::WindowsTemp
    };
    storage_paths::initialize_fixture(&app, &temp, mode, "cache");
    let paths = storage_paths::require_ready().unwrap();
    if std::env::var("YT_DLP_GUI_SUPPLEMENT_REAL").unwrap() == "1" {
        real_gallery(&app);
        return;
    }
    let persistent = app.join("data/cookies/sentinel.cookies");
    let unrelated = temp.join("yt-dlp-native-legacy.cookies");
    write(&persistent, b"synthetic persistent credential sentinel");
    write(&unrelated, b"unrelated old snapshot sentinel");
    let request = serde_json::json!({"url":"https://example.com/post", "browserCookies":[{
        "domain":"example.com", "hostOnly":true, "path":"/", "name":"synthetic", "value":"fixture-only", "secure":true
    }]});
    let snapshot = NativeCookies::from_request(&request).unwrap().unwrap();
    let snapshot_path = snapshot.path();
    assert!(snapshot_path.starts_with(paths.cache_root.join("temp/native-cookies")));
    assert!(
        fs::read_to_string(&snapshot_path)
            .unwrap()
            .contains("fixture-only")
    );
    assert!(storage_paths::exclusive_cleanup().is_err());
    run_storage_child(&app, &temp, "lock-busy");
    assert!(!portable_cleanup::cleanup_startup_transient_files().has_work());
    assert!(snapshot_path.exists());
    let first = storage_paths::CacheWorkDirectory::new().unwrap();
    let second = storage_paths::CacheWorkDirectory::new().unwrap();
    assert_ne!(first.path, second.path);
    write(&first.path.join("part.bin"), b"in-use gallery intermediate");
    // A later configuration view cannot change the initialized paths of either writer.
    storage_paths::initialize_fixture(&app, &temp, CacheLocationMode::WindowsTemp, "other-cache");
    assert_eq!(storage_paths::paths().cache_root, paths.cache_root);
    drop(snapshot);
    assert!(!snapshot_path.exists());
    assert!(storage_paths::exclusive_cleanup().is_err());
    let first_path = first.path.clone();
    drop(first);
    assert!(!first_path.exists());
    drop(second);
    assert!(storage_paths::exclusive_cleanup().is_ok());
    let cookie_root = paths.cache_root.join("temp/native-cookies");
    let old_cookie = cookie_root.join("crashed.cookies");
    let fresh_cookie = cookie_root.join("fresh.cookies");
    write(&old_cookie, b"synthetic abandoned snapshot");
    write(&fresh_cookie, b"synthetic fresh snapshot");
    stale(&old_cookie);
    let old_work = paths.cache_root.join("gallery-dl-temp/crashed-job");
    write(&old_work.join("part.bin"), b"abandoned gallery bytes");
    stale(&old_work);
    assert!(storage_paths::disposable_size(&paths.cache_root) > 0);
    let summary = portable_cleanup::cleanup_startup_transient_files();
    assert!(summary.errors.is_empty(), "{:?}", summary.errors);
    assert!(!old_cookie.exists());
    assert!(!old_work.exists());
    assert!(fresh_cookie.exists());
    // An invalid managed subdirectory fails before any fallback snapshot is created.
    storage_paths::remove_owned_tree(&cookie_root).unwrap();
    write(&cookie_root, b"not a directory");
    assert!(NativeCookies::from_request(&request).is_err());
    fs::remove_file(&cookie_root).unwrap();
    let cleanup = storage_paths::exclusive_cleanup().unwrap();
    assert!(NativeCookies::from_request(&request).is_err());
    assert!(storage_paths::CacheWorkDirectory::new().is_err());
    storage_paths::clear_disposable(paths, &[]).unwrap();
    drop(cleanup);
    assert_eq!(
        fs::read(persistent).unwrap(),
        b"synthetic persistent credential sentinel"
    );
    assert_eq!(
        fs::read(unrelated).unwrap(),
        b"unrelated old snapshot sentinel"
    );
}

fn all_files(path: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(all_files(&path));
            } else {
                result.push(path);
            }
        }
    }
    result
}

fn real_gallery(app: &Path) {
    use std::io::Read;
    use std::process::Stdio;
    let python = PathBuf::from(
        std::env::var_os("YT_DLP_GUI_TEST_PYTHON").expect("Set YT_DLP_GUI_TEST_PYTHON"),
    );
    let packages = std::env::var_os("YT_DLP_GUI_GALLERY_TEST_PACKAGES")
        .expect("Set YT_DLP_GUI_GALLERY_TEST_PACKAGES");
    let script = app.parent().unwrap().join("gallery-fixture.py");
    fs::write(&script, include_str!("storage_gallery_fixture.py")).unwrap();
    let postprocess_ffmpeg = std::env::var_os("YT_DLP_GUI_GALLERY_TEST_FFMPEG").map(PathBuf::from);
    let media = app.parent().unwrap().join("fixture-media");
    if let Some(ffmpeg) = &postprocess_ffmpeg {
        fs::create_dir_all(&media).unwrap();
        for (filename, filter, codec) in [
            ("fixture.mp4", "color=c=red:s=16x16:r=10", "mpeg4"),
            ("audio.m4a", "sine=frequency=440:sample_rate=44100", "aac"),
        ] {
            let output = Command::new(ffmpeg)
                .args([
                    "-v", "error", "-f", "lavfi", "-i", filter, "-t", "0.3", "-c", codec, "-y",
                ])
                .arg(media.join(filename))
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    let tools = ToolPaths {
        ffmpeg: postprocess_ffmpeg
            .as_ref()
            .cloned()
            .unwrap_or_else(|| app.join("missing-ffmpeg.exe"))
            .display()
            .to_string(),
        ..Default::default()
    };
    let standalone = std::env::var_os("YT_DLP_GUI_GALLERY_BUNDLE").map(PathBuf::from);
    let fixture_module = app.parent().unwrap().join("fixture-extractors");
    fs::create_dir_all(&fixture_module).unwrap();
    fs::write(
        fixture_module.join("storage_fixture.py"),
        include_str!("storage_gallery_fixture.py"),
    )
    .unwrap();
    let backend = GalleryDlBackend::new(if let Some(bundle) = &standalone {
        GalleryDlRuntime::standalone(bundle.join("gallery-dl.exe"))
    } else {
        GalleryDlRuntime::source_harness(&python)
    });
    let kinds: &[&str] = if postprocess_ffmpeg.is_some() {
        &["remux", "merged"]
    } else {
        &["http", "single", "playlist", "nested", "images"]
    };
    for &kind in kinds {
        let mode = if storage_paths::paths().active_mode == CacheLocationMode::WindowsTemp {
            "temporary"
        } else {
            "portable"
        };
        let destination = app.join(format!("final output 空间 %literal/{mode}/{kind}"));
        let mut prepared = backend
            .prepare_item_download(
                &format!("storagefixture:{kind}"),
                &destination,
                &tools,
                false,
            )
            .unwrap();
        if postprocess_ffmpeg.is_some() {
            prepared.command.env("YT_DLP_GUI_FIXTURE_MEDIA", &media);
            let mut options = serde_json::json!({
                "cachedir": storage_paths::paths().cache_root.join("yt-dlp"),
            });
            if kind == "remux" {
                options["postprocessors"] = serde_json::json!([
                    {"key":"FFmpegVideoRemuxer","preferedformat":"mkv"}
                ]);
            } else {
                options["format"] = serde_json::json!("video+audio");
            }
            prepared
                .command
                .arg("-o")
                .arg(format!("downloader.ytdl.raw-options={options}"));
        }
        let mut command = fixture_command(
            &prepared,
            &python,
            &script,
            &fixture_module,
            &packages,
            standalone.is_some(),
        );
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut process = command.spawn().unwrap();
        let mut stdout = process.stdout.take().unwrap();
        let mut stderr = process.stderr.take().unwrap();
        let out_reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).unwrap();
            bytes
        });
        let err_reader = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).unwrap();
            bytes
        });
        let mut observed = std::collections::BTreeSet::new();
        let started = std::time::Instant::now();
        let status = loop {
            observed.extend(all_files(app.parent().unwrap()).into_iter().filter(|path| {
                path.to_string_lossy().contains(".part")
                    || path.starts_with(&prepared.storage_work.path)
            }));
            if let Some(status) = process.try_wait().unwrap() {
                break status;
            }
            if started.elapsed() > Duration::from_secs(40) {
                process.kill().unwrap();
                panic!("Gallery tool timed out")
            }
            std::thread::sleep(Duration::from_millis(3));
        };
        let stdout = out_reader.join().unwrap();
        let stderr = err_reader.join().unwrap();
        fs::write(
            app.parent()
                .unwrap()
                .join(format!("gallery-{mode}-{kind}-stdout.log")),
            &stdout,
        )
        .unwrap();
        fs::write(
            app.parent()
                .unwrap()
                .join(format!("gallery-{mode}-{kind}-stderr.log")),
            &stderr,
        )
        .unwrap();
        fs::write(
            app.parent()
                .unwrap()
                .join(format!("gallery-{mode}-{kind}-observed.json")),
            serde_json::to_vec_pretty(&observed).unwrap(),
        )
        .unwrap();
        assert!(
            status.success(),
            "{kind}: {}",
            String::from_utf8_lossy(&stderr)
        );
        println!("{kind}: {}", String::from_utf8_lossy(&stderr));
        assert!(
            !observed.is_empty() || kind == "images",
            "No actual intermediate file observed: {kind}"
        );
        assert!(
            observed
                .iter()
                .all(|p| p.starts_with(&prepared.storage_work.path)),
            "Intermediate escaped the active gallery work directory: {observed:?}"
        );
        let events: Vec<_> = String::from_utf8_lossy(&stdout)
            .lines()
            .filter_map(crate::infrastructure::gallery_dl::parse_gallery_dl_download_event)
            .collect();
        assert!(
            !events.is_empty(),
            "No final output event for {kind}: {}",
            String::from_utf8_lossy(&stdout)
        );
        let expected = if matches!(kind, "playlist" | "nested" | "images" | "remux" | "merged") {
            3
        } else {
            1
        };
        assert_eq!(events.len(), expected, "{kind} artifact count");
        let mut paths = std::collections::BTreeSet::new();
        let mut established = std::collections::BTreeMap::new();
        for event in events {
            let event = event.unwrap();
            assert!(event.artifact.path.starts_with(&destination));
            if let Some(ffmpeg) = &postprocess_ffmpeg {
                assert_eq!(
                    event.artifact.path.extension().unwrap(),
                    if kind == "remux" { "mkv" } else { "mp4" }
                );
                assert_eq!(
                    decoded_video(ffmpeg, &event.artifact.path),
                    decoded_video(ffmpeg, &media.join("fixture.mp4"))
                );
                if kind == "merged" {
                    let probe = Command::new(ffmpeg.with_file_name("ffprobe.exe"))
                        .args([
                            "-v",
                            "error",
                            "-select_streams",
                            "a",
                            "-show_entries",
                            "stream=codec_name",
                            "-of",
                            "json",
                        ])
                        .arg(&event.artifact.path)
                        .output()
                        .unwrap();
                    assert!(probe.status.success());
                    let probe: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
                    assert_eq!(probe["streams"][0]["codec_name"], "aac");
                }
            } else if kind == "images" {
                use crate::infrastructure::gallery_xmp::{
                    GalleryXmpWriteOutcome, write_gallery_xmp,
                };
                assert!(
                    fs::read(&event.artifact.path)
                        .unwrap()
                        .starts_with(b"\x89PNG\r\n\x1a\n")
                );
                let fields = crate::app::gallery_canonical_metadata::map_gallery_artifact_metadata(
                    &event.metadata,
                    &event.artifact.path,
                );
                assert_eq!(
                    write_gallery_xmp(&event.artifact.path, &event.artifact.archive_key, &fields)
                        .unwrap(),
                    GalleryXmpWriteOutcome::Written
                );
                assert_eq!(
                    write_gallery_xmp(&event.artifact.path, &event.artifact.archive_key, &fields)
                        .unwrap(),
                    GalleryXmpWriteOutcome::AlreadyCurrent
                );
            } else {
                assert_eq!(
                    fs::read(&event.artifact.path).unwrap(),
                    vec![b'S'; 256 * 1024]
                );
            }
            established.insert(
                event.artifact.path.clone(),
                fs::read(&event.artifact.path).unwrap(),
            );
            assert!(paths.insert(event.artifact.path));
            assert_eq!(event.artifact.archive_key, "storagefixturefixture");
        }
        assert!(
            String::from_utf8_lossy(&stdout)
                .contains(crate::infrastructure::gallery_dl::GALLERY_POST_COMPLETED_MARKER)
        );
        // A second run must skip existing outputs without media transfer.
        let skipped = fixture_command(
            &prepared,
            &python,
            &script,
            &fixture_module,
            &packages,
            standalone.is_some(),
        )
        .output()
        .unwrap();
        assert!(
            skipped.status.success(),
            "{}",
            String::from_utf8_lossy(&skipped.stderr)
        );
        assert!(
            String::from_utf8_lossy(&skipped.stderr).contains("fixture-transfers={\"get\": 0}")
        );
        assert_eq!(
            String::from_utf8_lossy(&skipped.stdout)
                .lines()
                .filter_map(crate::infrastructure::gallery_dl::parse_gallery_dl_download_event)
                .count(),
            expected
        );
        for (path, bytes) in established {
            assert_eq!(
                fs::read(path).unwrap(),
                bytes,
                "Existing output changed during skip"
            );
        }
        if kind != "http" {
            assert!(
                storage_paths::paths()
                    .cache_root
                    .join(format!("yt-dlp/storage-policy/{kind}.json"))
                    .is_file()
            );
        }
        assert!(storage_paths::exclusive_cleanup().is_err());
        let work = prepared.storage_work.path.clone();
        // Exit alone does not release the lease; the prepared execution owner does.
        prepared.command = Command::new(&python);
        drop(prepared);
        assert!(!work.exists());
        assert!(storage_paths::exclusive_cleanup().is_ok());
    }
    if postprocess_ffmpeg.is_some() {
        return;
    }
    let mode = if storage_paths::paths().active_mode == CacheLocationMode::WindowsTemp {
        "temporary"
    } else {
        "portable"
    };
    for kind in ["metadata", "failed", "collision"] {
        let destination = app.join(format!("special output/{mode}/{kind}"));
        let url = if kind == "metadata" {
            "ytdl:storagepolicy:metadata"
        } else if kind == "failed" {
            "storagefixture:failed"
        } else {
            "storagefixture:collision"
        };
        let mut prepared = backend
            .prepare_item_download(url, &destination, &tools, false)
            .unwrap();
        if kind == "metadata" {
            for option in [
                "download=false",
                "directory-create=false",
                "output.mode=false",
                "cookies-update=false",
            ] {
                prepared.command.arg("-o").arg(option);
            }
        } else {
            prepared.command.arg("-o").arg("downloader.ytdl.retries=0");
        }
        let output = fixture_command(
            &prepared,
            &python,
            &script,
            &fixture_module,
            &packages,
            standalone.is_some(),
        )
        .output()
        .unwrap();
        fs::write(
            app.parent()
                .unwrap()
                .join(format!("gallery-{mode}-{kind}-stdout.log")),
            &output.stdout,
        )
        .unwrap();
        fs::write(
            app.parent()
                .unwrap()
                .join(format!("gallery-{mode}-{kind}-stderr.log")),
            &output.stderr,
        )
        .unwrap();
        if kind == "metadata" {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(!destination.exists());
            assert!(all_files(&prepared.storage_work.path).is_empty());
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("fixture-transfers={\"get\": 0}")
            );
            assert!(
                storage_paths::paths()
                    .cache_root
                    .join("yt-dlp/storage-policy/metadata.json")
                    .exists()
            );
        } else {
            assert!(!output.status.success());
            assert!(
                !String::from_utf8_lossy(&output.stdout)
                    .contains(crate::infrastructure::gallery_dl::GALLERY_POST_COMPLETED_MARKER)
            );
            let events: Vec<_> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(crate::infrastructure::gallery_dl::parse_gallery_dl_download_event)
                .collect();
            assert_eq!(
                events.len(),
                if kind == "collision" { 0 } else { 2 },
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stderr).contains(if kind == "collision" {
                    "fixture-transfers={\"get\": 0}"
                } else {
                    "fixture-transfers={\"get\": 3}"
                })
            );
            for event in events {
                let event = event.unwrap();
                assert_eq!(
                    fs::read(event.artifact.path).unwrap(),
                    vec![b'S'; 256 * 1024]
                );
            }
        }
        println!("{kind}: expected outcome verified ({mode})");
    }
    let destination = app.join(format!("special output/{mode}/cancelled"));
    let prepared = backend
        .prepare_item_download("storagefixture:playlist", &destination, &tools, false)
        .unwrap();
    let work = prepared.storage_work.path.clone();
    let mut command = fixture_command(
        &prepared,
        &python,
        &script,
        &fixture_module,
        &packages,
        standalone.is_some(),
    );
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let started = std::time::Instant::now();
    loop {
        if !all_files(&destination).is_empty() {
            child.kill().unwrap();
            break;
        }
        assert!(
            child.try_wait().unwrap().is_none(),
            "Playlist finished before cancellation boundary"
        );
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "No completed playlist entry before cancellation timeout"
        );
        std::thread::sleep(Duration::from_millis(3));
    }
    child.wait().unwrap();
    let output = out.join().unwrap();
    let errors = err.join().unwrap();
    fs::write(
        app.parent()
            .unwrap()
            .join(format!("gallery-{mode}-cancelled-stdout.log")),
        &output,
    )
    .unwrap();
    fs::write(
        app.parent()
            .unwrap()
            .join(format!("gallery-{mode}-cancelled-stderr.log")),
        errors,
    )
    .unwrap();
    assert!(
        !String::from_utf8_lossy(&output)
            .contains(crate::infrastructure::gallery_dl::GALLERY_POST_COMPLETED_MARKER)
    );
    let completed = all_files(&destination);
    assert!(!completed.is_empty() && completed.len() < 3);
    for path in completed {
        assert_eq!(fs::read(path).unwrap(), vec![b'S'; 256 * 1024]);
    }
    assert!(storage_paths::exclusive_cleanup().is_err());
    drop(prepared);
    assert!(!work.exists());
    println!("cancelled: completed files retained; owned work released ({mode})");
}

fn decoded_video(ffmpeg: &Path, path: &Path) -> Vec<u8> {
    let output = Command::new(ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-map", "0:v:0", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!output.stdout.is_empty());
    output.stdout
}

fn fixture_command(
    prepared: &crate::infrastructure::gallery_dl::PreparedGalleryDlDownload,
    python: &Path,
    script: &Path,
    modules: &Path,
    packages: &std::ffi::OsStr,
    standalone: bool,
) -> Command {
    let args: Vec<_> = prepared.command.get_args().map(|a| a.to_owned()).collect();
    let mut command = if standalone {
        let mut command = Command::new(prepared.command.get_program());
        command.args(args).arg("-X").arg(modules);
        command
    } else {
        let mut command = Command::new(python);
        command.arg("-B").arg(script).args(args);
        command
    };
    for (key, value) in prepared.command.get_envs() {
        if let Some(value) = value {
            command.env(key, value);
        } else {
            command.env_remove(key);
        }
    }
    if standalone {
        command.env_remove("PYTHONPATH");
    } else {
        command.env("PYTHONPATH", packages);
    }
    command
}
