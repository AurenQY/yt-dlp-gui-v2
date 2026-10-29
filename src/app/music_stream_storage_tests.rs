//! R-CACHE-001: actual detached producers outlive a stopped playback owner.
use super::*;
use crate::infrastructure::{CacheLocationMode, ToolPaths, storage_paths};
use std::net::TcpListener;
use std::sync::mpsc;

#[test]
#[ignore = "Set YT_DLP_GUI_TEST_YTDLP; localhost-only slow detached producers"]
fn detached_producers_keep_storage_after_playback_stops() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/cache-location-validation/playback-review-{}",
        std::process::id()
    ));
    for mode in ["portable", "temporary"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "app::music_stream::storage_tests::detached_producer_child",
                "--nocapture",
            ])
            .env("YT_DLP_GUI_PLAYBACK_REVIEW_ROOT", root.join(mode))
            .env("YT_DLP_GUI_PLAYBACK_REVIEW_MODE", mode)
            .output()
            .unwrap();
        println!("{}", String::from_utf8_lossy(&output.stdout));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    println!("R-CACHE-001 fixture: {}", root.display());
}

#[test]
fn detached_producer_child() {
    let Some(root) = std::env::var_os("YT_DLP_GUI_PLAYBACK_REVIEW_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let app = root.join("app");
    let temp = root.join("system-temp");
    fs::create_dir_all(&app).unwrap();
    fs::create_dir_all(&temp).unwrap();
    let mode = if std::env::var("YT_DLP_GUI_PLAYBACK_REVIEW_MODE").unwrap() == "portable" {
        CacheLocationMode::V2Cache
    } else {
        CacheLocationMode::WindowsTemp
    };
    storage_paths::initialize_fixture(&app, &temp, mode, "cache");
    for kind in ["http", "yt-dlp"] {
        slow_detached(&root, kind);
    }
}

fn slow_detached(root: &Path, kind: &str) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (arrived_tx, arrived_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let tool = kind == "yt-dlp";
    let server = thread::spawn(move || {
        let mut finished = false;
        let mut successful_requests = 0;
        while !finished {
            let (mut connection, _) = listener.accept().unwrap();
            connection
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") {
                assert!(request.len() < 16384);
                assert_eq!(connection.read(&mut byte).unwrap(), 1);
                request.push(byte[0]);
            }
            if request.starts_with(b"HEAD ") {
                write!(connection, "HTTP/1.1 200 OK\r\nContent-Length: 1048576\r\nContent-Type: video/mp4\r\nConnection: close\r\n\r\n").unwrap();
                continue;
            }
            if tool && String::from_utf8_lossy(&request).contains("/denied.mp4") {
                arrived_tx.send("cookie-attempt").unwrap();
                release_rx.recv_timeout(Duration::from_secs(20)).unwrap();
                write!(
                    connection,
                    "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                continue;
            }
            write!(connection, "HTTP/1.1 200 OK\r\nContent-Length: 1048576\r\nContent-Type: video/mp4\r\nConnection: close\r\n\r\n").unwrap();
            successful_requests += 1;
            // Generic extraction probes the URL before its media transfer.
            if tool && successful_requests == 1 {
                let _ = connection.write_all(&vec![0x53; 1024 * 1024]);
                continue;
            }
            connection.write_all(&vec![0x53; 32 * 1024]).unwrap();
            connection.flush().unwrap();
            arrived_tx.send("transfer").unwrap();
            release_rx.recv_timeout(Duration::from_secs(20)).unwrap();
            connection
                .write_all(&vec![0x53; 1024 * 1024 - 32 * 1024])
                .unwrap();
            finished = true;
        }
    });
    let cache = storage_paths::paths().cache_root.join("audio");
    let media = cache.join(kind).join("audio.mp4");
    let tools = ToolPaths {
        yt_dlp: std::env::var("YT_DLP_GUI_TEST_YTDLP").unwrap_or_default(),
        ..Default::default()
    };
    let command = |name| {
        tools
            .prepare_music_stream_cache_command(
                &format!("http://{address}/{name}.mp4"),
                &media,
                "best",
                false,
            )
            .unwrap()
    };
    let stream = ResolvedMusicStream {
        item_id: 1,
        session_id: 1,
        source_url: format!("http://{address}/fixture.mp4"),
        direct_url: format!("http://{address}/fixture.mp4"),
        headers: Vec::new(),
        title: "synthetic".into(),
        album_title: String::new(),
        thumbnail_url: String::new(),
        duration_seconds: None,
        ext: "mp4".into(),
        format_id: "fixture".into(),
        acodec: String::new(),
        cache_key: kind.into(),
        expected_bytes: Some(1024 * 1024),
        cache_root: cache,
        cache_command: tool.then(|| command("denied")),
        cache_cookie_free_retry_command: tool.then(|| command("fixture")),
        volume: 0.0,
    };
    let shared = Arc::new(SharedPlaybackState::new(0.0, None, 1, 1));
    shared.stop_requested.store(true, Ordering::Relaxed);
    let state = Arc::new(CacheTransferState::default());
    let (events, _) = mpsc::channel();
    run_stream_playback(stream, shared, state.clone(), events).unwrap();
    // The actual owner has returned. The producer's guard must already exist,
    // including before its new thread is scheduled and across cookie retry.
    assert!(storage_paths::exclusive_cleanup().is_err());
    if tool {
        assert_eq!(
            arrived_rx.recv_timeout(Duration::from_secs(20)).unwrap(),
            "cookie-attempt"
        );
        assert!(storage_paths::exclusive_cleanup().is_err());
        release_tx.send(()).unwrap();
    }
    assert_eq!(
        arrived_rx.recv_timeout(Duration::from_secs(20)).unwrap(),
        "transfer"
    );
    let started = Instant::now();
    while fs::metadata(&media).map(|m| m.len()).unwrap_or(0) == 0 {
        assert!(started.elapsed() < Duration::from_secs(10));
        thread::sleep(Duration::from_millis(10));
    }
    let before = fs::read(&media).unwrap();
    assert!(storage_paths::exclusive_cleanup().is_err());
    assert_eq!(fs::read(&media).unwrap(), before);
    release_tx.send(()).unwrap();
    server.join().unwrap();
    let started = Instant::now();
    loop {
        if let Ok(guard) = storage_paths::exclusive_cleanup() {
            assert!(
                state.complete.load(Ordering::Relaxed),
                "{:?}",
                state.error_text()
            );
            assert_eq!(fs::read(&media).unwrap(), vec![0x53; 1024 * 1024]);
            fs::write(root.join(format!("{kind}-complete.json")), serde_json::to_vec(&serde_json::json!({"owner":"stopped","bytes":1048576,"cookie_retry":tool,"cleanup":"busy until producer settled"})).unwrap()).unwrap();
            storage_paths::clear_disposable(storage_paths::paths(), &[]).unwrap();
            drop(guard);
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(20));
        thread::sleep(Duration::from_millis(20));
    }
    assert!(!media.exists());
    println!(
        "{kind}: stopped owner, live transfer/retry protected, final bytes verified, cleanup released"
    );
}
