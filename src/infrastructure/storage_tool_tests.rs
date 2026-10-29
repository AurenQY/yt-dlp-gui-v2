//! Opt-in filesystem verification using a real yt-dlp executable and localhost only.
use super::fixture;
use crate::infrastructure::{storage_paths, tools};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

#[test]
#[ignore = "Set YT_DLP_GUI_TEST_YTDLP to an installed executable; uses localhost only"]
fn storage_real_tool_placement() {
    let source = std::env::var_os("YT_DLP_GUI_TEST_YTDLP").expect("Set YT_DLP_GUI_TEST_YTDLP");
    let (app, temp) = fixture("real-tool");
    let tool = app.parent().unwrap().join("tool-bin/yt-dlp.exe");
    fs::create_dir_all(tool.parent().unwrap()).unwrap();
    fs::copy(source, &tool).unwrap();
    for phase in ["portable", "temporary"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "infrastructure::storage_paths::tests::real_tools::storage_real_tool_child",
                "--ignored",
                "--nocapture",
            ])
            .env("YT_DLP_GUI_TEST_YTDLP", &tool)
            .env("YT_DLP_GUI_STORAGE_TEST_APP", &app)
            .env("YT_DLP_GUI_STORAGE_TEST_TEMP", &temp)
            .env("YT_DLP_GUI_STORAGE_TEST_PHASE", phase)
            .env("TMP", &temp)
            .env("TEMP", &temp)
            .output()
            .unwrap();
        println!("{}", String::from_utf8_lossy(&output.stdout));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
    }
    println!("Real-tool fixture: {}", app.parent().unwrap().display());
}

#[test]
#[ignore = "Internal subprocess for the real-tool test"]
fn storage_real_tool_child() {
    let app = std::path::PathBuf::from(std::env::var_os("YT_DLP_GUI_STORAGE_TEST_APP").unwrap());
    let temp = std::path::PathBuf::from(std::env::var_os("YT_DLP_GUI_STORAGE_TEST_TEMP").unwrap());
    let phase = std::env::var("YT_DLP_GUI_STORAGE_TEST_PHASE").unwrap();
    let tool = std::env::var_os("YT_DLP_GUI_TEST_YTDLP").unwrap();
    let mode = if phase == "portable" {
        tools::CacheLocationMode::V2Cache
    } else {
        tools::CacheLocationMode::WindowsTemp
    };
    storage_paths::initialize_fixture(&app, &temp, mode, "cache");
    let paths = storage_paths::require_ready().unwrap();
    let version = Command::new(&tool)
        .args(["--ignore-config", "--version"])
        .output()
        .unwrap();
    assert!(version.status.success());
    println!(
        "yt-dlp version: {}",
        String::from_utf8_lossy(&version.stdout).trim()
    );

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let payload = Arc::new(vec![0x4a; 256 * 1024]);
    let response_body = payload.clone();
    let server = std::thread::spawn(move || {
        while !stopping.load(Ordering::Relaxed) {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
            let mut request = Vec::new();
            let mut byte = [0];
            while request.len() < 16_384 && !request.ends_with(b"\r\n\r\n") {
                if stream.read(&mut byte).unwrap_or(0) == 0 {
                    break;
                }
                request.push(byte[0]);
            }
            let head = request.starts_with(b"HEAD ");
            if write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response_body.len()).is_err() { continue; }
            if !head {
                for chunk in response_body.chunks(2048) {
                    if stream.write_all(chunk).is_err() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(3));
                }
            }
        }
    });
    let tool_paths = tools::ToolPaths {
        yt_dlp: std::path::PathBuf::from(&tool).display().to_string(),
        live_from_start: false,
        ..Default::default()
    };
    for explicit in [false, true] {
        let output_dir = app.join(format!("output 100% 路径/{phase}"));
        let name = if explicit {
            "absolute clip.mp4"
        } else {
            "ordinary clip.mp4"
        };
        let destination = output_dir.join(name);
        let request = tools::DownloadRequest {
            output: Default::default(),
            target_kind: tools::DownloadTargetKind::Video,
            url: format!("http://127.0.0.1:{port}/fixture.mp4"),
            playlist_index: None,
            format_selector: "best".to_owned(),
            video_selector: "best".to_owned(),
            audio_selector: String::new(),
            is_muxed_video: true,
            video_ext: "mp4".to_owned(),
            audio_ext: String::new(),
            merge_output_container: None,
            output_extension_override: None,
            upload_date: String::new(),
            subtitle_lang: None,
            subtitle_ext: "srt".to_owned(),
            subtitle_source_ext: String::new(),
            subtitle_url: None,
            write_auto_subs: false,
            subtitle_is_auto_translated: false,
            write_subtitles: false,
            embed_subtitles: false,
            write_chapters: false,
            embed_chapters: false,
            write_thumbnail: false,
            embed_thumbnail: false,
            use_cookies: false,
            use_aria2: false,
            emit_json: false,
            output_path: explicit.then(|| destination.display().to_string()),
            output_dir: output_dir.display().to_string(),
            file_name: name.to_owned(),
            download_section_args: Vec::new(),
        };
        let _guard = storage_paths::use_cache().unwrap();
        let mut prepared = tool_paths.prepare_download(&request).unwrap();
        let log = app.join(format!(
            "{phase}-{}.log",
            if explicit { "absolute" } else { "ordinary" }
        ));
        println!("{}", prepared.command_line);
        let output_file = fs::File::create(&log).unwrap();
        prepared
            .command
            .stdout(Stdio::from(output_file.try_clone().unwrap()))
            .stderr(Stdio::from(output_file));
        let mut child = prepared.command.spawn().unwrap();
        let started = Instant::now();
        let mut observed_intermediate = false;
        let status = loop {
            observed_intermediate |= contains_file(&paths.cache_root.join("yt-dlp-temp"));
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if started.elapsed() > Duration::from_secs(40) {
                let _ = child.kill();
                let _ = child.wait();
                panic!("yt-dlp fixture timed out: {}", log.display());
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(status.success(), "{}", fs::read_to_string(&log).unwrap());
        prepared
            .output_transaction
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .publish(&prepared.output_path, &AtomicBool::new(false))
            .unwrap();
        assert!(
            observed_intermediate,
            "No intermediate file observed below the active cache: {}",
            log.display()
        );
        assert_eq!(fs::read(&destination).unwrap(), *payload);
        let log_text = fs::read_to_string(&log).unwrap();
        assert!(
            log_text.contains(tools::FINAL_OUTPUT_PATH_PREFIX),
            "No final-path report"
        );
        fs::write(
            output_dir.join("user-notes.txt"),
            b"unknown neighbor of completed media",
        )
        .unwrap();
        drop(_guard);
        drop(prepared);
        let cleanup = storage_paths::exclusive_cleanup().unwrap();
        storage_paths::clear_disposable(paths, &[]).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), *payload);
        assert_eq!(
            fs::read(output_dir.join("user-notes.txt")).unwrap(),
            b"unknown neighbor of completed media"
        );
        drop(cleanup);
        println!(
            "Verified {phase} / explicit={explicit}: intermediate under {}; final {}",
            paths.cache_root.display(),
            destination.display()
        );
    }
    if phase == "temporary" {
        let previous = app.join("output 100% 路径/portable");
        for name in ["ordinary clip.mp4", "absolute clip.mp4"] {
            assert_eq!(fs::read(previous.join(name)).unwrap(), *payload);
        }
    }
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
}

fn contains_file(path: &Path) -> bool {
    let Ok(entries) = fs::read_dir(path) else {
        return false;
    };
    entries
        .filter_map(Result::ok)
        .any(|entry| entry.path().is_file() || contains_file(&entry.path()))
}
