//! Opt-in real-tool contracts. Local HTTP only, no account or public download.
use super::*;
use crate::app::download_worker::DownloadResult;
use crate::infrastructure::{output_policy::*, storage_paths as storage};

#[path = "output_review_resolve_log_tests.rs"]
mod review_resolve_log;
#[path = "output_review_metadata_tests.rs"]
mod review_metadata;
#[path = "output_review_guard_tests.rs"]
mod review_guard;
#[path = "output_review_followup_tests.rs"]
mod review_followup;
#[path = "output_review_round1_tests.rs"]
mod review_round1;
#[path = "output_review_round2_tests.rs"]
mod review_round2;
#[path = "output_review_round3_tests.rs"]
mod review_round3;
#[path = "output_review_round4_tests.rs"]
mod review_round4;
#[path = "output_review_simplicity_tests.rs"]
mod review_simplicity;

#[path = "output_review_round5_tests.rs"]
mod review_round5;

#[test]
#[ignore = "Set YT_DLP_GUI_TEST_YTDLP and YT_DLP_GUI_TEST_FFMPEG; localhost only"]
fn output_actual_tools() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/output-validation")
        .join(format!(
            "actual-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
    for mode in ["portable", "temporary"] {
        let run = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "app::state::output_review_tests::output_actual_child",
                "--nocapture",
            ])
            .env("OUTPUT_ACTUAL_ROOT", root.join(mode))
            .output()
            .unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(format!("{mode}.stdout.log")), &run.stdout).unwrap();
        fs::write(root.join(format!("{mode}.stderr.log")), &run.stderr).unwrap();
        println!("{}", String::from_utf8_lossy(&run.stdout));
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
    println!("Actual output evidence: {}", root.display());
}

struct Server {
    address: String,
    root: PathBuf,
    child: Child,
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Server {
    fn media_get_count(&self) -> usize {
        fs::read_to_string(self.root.join("media-get-count.txt"))
            .unwrap()
            .parse()
            .unwrap()
    }
}
fn server(root: &Path, audio: &Path) -> Server {
    fs::copy(audio, root.join("source.wav")).unwrap();
    let mut command = Command::new("python");
    configure_background_command(&mut command);
    let child = command
        .args(["-u", "-B"])
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/app/state/output_review_http_fixture.py"),
        )
        .arg(root)
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .stdout(Stdio::from(
            fs::File::create(root.join("http.stdout.log")).unwrap(),
        ))
        .stderr(Stdio::from(
            fs::File::create(root.join("http.stderr.log")).unwrap(),
        ))
        .spawn()
        .unwrap();
    let started = Instant::now();
    let port = loop {
        if let Ok(port) = fs::read_to_string(root.join("http-port.txt")) {
            if let Ok(port) = port.parse::<u16>() {
                break port;
            }
        }
        assert!(started.elapsed() < Duration::from_secs(10));
        thread::sleep(Duration::from_millis(20));
    };
    Server {
        address: format!("http://127.0.0.1:{port}"),
        root: root.to_owned(),
        child,
    }
}

fn request(url: &str, destination: &Path, title: &str) -> DownloadRequest {
    DownloadRequest {
        output: OutputRequest {
            automatic: true,
            context: OutputNameContext {
                resolved: true,
                title: title.into(),
                id: url.rsplit('/').next().unwrap().into(),
                uploader: "Fixture uploader".into(),
                upload_date: "20261027".into(),
                source: url.into(),
            },
            ..Default::default()
        },
        target_kind: DownloadTargetKind::Normal,
        url: url.into(),
        playlist_index: None,
        format_selector: "best".into(),
        video_selector: "best".into(),
        audio_selector: String::new(),
        is_muxed_video: true,
        video_ext: "mp4".into(),
        audio_ext: "m4a".into(),
        merge_output_container: None,
        output_extension_override: None,
        upload_date: String::new(),
        subtitle_lang: None,
        subtitle_ext: "srt".into(),
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
        output_path: None,
        output_dir: destination.display().to_string(),
        file_name: String::new(),
        download_section_args: Vec::new(),
    }
}
fn download(tools: &ToolPaths, request: DownloadRequest) -> DownloadResult {
    let (tx, rx) = mpsc::channel();
    run_download_worker(
        tools.clone(),
        request,
        1,
        1,
        WorkflowKind::DownloadMedia,
        tx,
        Arc::new(Mutex::new(None)),
        Arc::new(AtomicBool::new(false)),
    );
    rx.into_iter()
        .find_map(|e| {
            if let DownloadEvent::Finished(result) = e {
                Some(result)
            } else {
                None
            }
        })
        .unwrap()
}
fn music(
    tools: &ToolPaths,
    source: Option<&Path>,
    url: &str,
    destination: &Path,
    title: &str,
    format: MusicDownloadFormat,
    chapters: bool,
) -> (OutputDisposition, Result<String, String>, bool) {
    let choice = MusicDownloadChoice {
        mode: MusicDownloadMode::Unified,
        unified_format: format,
        embed_cover: false,
        write_tags: false,
        save_mode: if chapters {
            MusicDownloadSaveMode::SplitChapters
        } else {
            MusicDownloadSaveMode::FullAudio
        },
        ..Default::default()
    };
    let (tx, rx) = mpsc::channel();
    let job = MusicDownloadJob {
        output: OutputRequest {
            automatic: true,
            context: OutputNameContext {
                resolved: true,
                title: title.into(),
                id: "audio-id".into(),
                source: url.into(),
                ..Default::default()
            },
            ..Default::default()
        },
        item_id: 1,
        workflow_id: 1,
        source_url: url.into(),
        title: title.into(),
        album_title: String::new(),
        output_dir: destination.into(),
        choice,
        source_acodec: "pcm_s16le".into(),
        cache_media_path: source.map(Path::to_owned),
        cover_path: None,
        cover_cache_dir: None,
        thumbnail_url: String::new(),
        use_cookies: false,
    };
    run_music_download_worker(
        tools.clone(),
        job,
        tx,
        Arc::new(Mutex::new(None)),
        Arc::new(AtomicBool::new(false)),
    );
    rx.into_iter()
        .find_map(|e| {
            if let MusicDownloadEvent::Finished {
                disposition,
                result,
                chapters_split,
                ..
            } = e
            {
                Some((disposition, result, chapters_split))
            } else {
                None
            }
        })
        .unwrap()
}
fn published(result: DownloadResult) -> PathBuf {
    assert_eq!(
        result.disposition,
        OutputDisposition::Published,
        "{:?}",
        result.result
    );
    PathBuf::from(result.result.unwrap())
}

#[test]
fn output_actual_child() {
    let Some(root) = std::env::var_os("OUTPUT_ACTUAL_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let app = root.join("app");
    let temp = root.join("system-temp");
    fs::create_dir_all(&app).unwrap();
    fs::create_dir_all(&temp).unwrap();
    let mode = if root.file_name().unwrap() == "temporary" {
        CacheLocationMode::WindowsTemp
    } else {
        CacheLocationMode::V2Cache
    };
    storage::initialize_fixture(&app, &temp, mode, "cache");
    let local_tool = app.join("tool-bin/yt-dlp.exe");
    fs::create_dir_all(local_tool.parent().unwrap()).unwrap();
    fs::copy(std::env::var("YT_DLP_GUI_TEST_YTDLP").unwrap(), &local_tool).unwrap();
    let tools = ToolPaths {
        yt_dlp: local_tool.display().to_string(),
        ffmpeg: std::env::var("YT_DLP_GUI_TEST_FFMPEG").unwrap(),
        live_from_start: false,
        ..Default::default()
    };
    let audio = storage::cache_directory("audio")
        .unwrap()
        .join("source.wav");
    let video = root.join("source.mp4");
    for (path, video) in [(&audio, false), (&video, true)] {
        let mut command = Command::new(&tools.ffmpeg);
        command.args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=0.6",
        ]);
        if video {
            command.args([
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=64x64:d=0.6",
                "-c:v",
                "mpeg4",
                "-c:a",
                "aac",
                "-shortest",
            ]);
        }
        let generated = command.arg(path).output().unwrap();
        assert!(
            generated.status.success(),
            "{}",
            String::from_utf8_lossy(&generated.stderr)
        );
    }
    let server = server(&root, &audio);
    let destination = root.join("final 100% 空间");
    let url_a = format!("{}/a.mp4", server.address);
    let url_b = format!("{}/b.mp4", server.address);
    if std::env::var_os("OUTPUT_REVIEW_RESOLVE_LOG").is_some() {
        review_resolve_log::run(&root, &tools, &server);
        return;
    }
    if std::env::var_os("OUTPUT_REVIEW_METADATA").is_some() {
        review_metadata::run(&root, &tools, &server);
        return;
    }
    if std::env::var_os("OUTPUT_REVIEW_GUARD").is_some() {
        review_guard::run(&root, &tools, &video, &server);
        return;
    }
    if std::env::var_os("OUTPUT_REVIEW_FOLLOWUP").is_some() {
        review_followup::run(&root, &tools, &video, &server);
        return;
    }
    if std::env::var_os("OUTPUT_REVIEW_SIMPLICITY").is_some() {
        review_simplicity::run(&root, &tools, &audio, &server);
        return;
    }
    if std::env::var_os("OUTPUT_REVIEW_ROUND5").is_some() {
        review_round5::run(&root, &tools, &audio, &video, &server);
        return;
    }
    if std::env::var_os("OUTPUT_REVIEW_ROUND4").is_some() {
        review_round4::run(&root, &tools, &audio, &video, &server);
        return;
    }
    if std::env::var_os("OUTPUT_REVIEW_ROUND3").is_some() {
        review_round3::run(&root, &tools, &audio, &server);
        return;
    }
    if std::env::var_os("OUTPUT_REVIEW_ROUND2").is_some() {
        review_round2::run(&root, &tools, &audio, &video, &server);
        return;
    }
    if std::env::var_os("OUTPUT_REVIEW_ROUND1").is_some() {
        review_round1::run(&root, &tools, &audio, &video, &server);
        return;
    }
    if std::env::var_os("OUTPUT_MANUAL_ONLY").is_some() {
        // Exercise the actual batch edit handler and dispatch snapshot against a
        // conflicting global naming rule, then run the real output pipeline.
        let mut config = AppConfig::default().with_fixture_file(app.join("manual-settings.yaml"));
        config.output_naming = OutputNamingMode::Custom;
        config.output_template = "Global {title}".into();
        let mut state = AppState::from_runtime(config, tools.clone());
        let mut item = state.build_queue_item_from_url(&url_a);
        item.selection.file_name = "Manual batch 100%".into();
        assert!(item.selection.name_origin == Some(crate::domain::OutputNameOrigin::Automatic));
        state.queue_items = vec![item];
        state.normalize_item_download_base_name(0);
        let output = state.output_request_for_item(&state.queue_items[0], false);
        assert!(!output.automatic);
        assert_eq!(output.literal_name, "Manual batch 100%");
        let mut task = request(&url_a, &destination, "Ignored automatic title");
        task.file_name = state.queue_items[0].selection.file_name.clone();
        task.output = output;
        let path = published(download(&tools, task));
        assert_eq!(path.file_name().unwrap(), "Manual batch 100%.mp4");
        println!(
            "{mode:?}: batch edit handler, explicit snapshot and real yt-dlp publication passed; media GETs={}",
            server.media_get_count()
        );
        return;
    }
    let original = published(download(
        &tools,
        request(&url_a, &destination, "Same title"),
    ));
    let original_hash = crate::infrastructure::sha256_hex_file(&original).unwrap();
    for (url, suffix) in [(&url_b, " (2)"), (&url_a, " (3)")] {
        let path = published(download(&tools, request(url, &destination, "Same title")));
        assert_eq!(
            path.file_stem().unwrap(),
            format!("Same title{suffix}").as_str()
        );
        assert_eq!(
            crate::infrastructure::sha256_hex_file(&original).unwrap(),
            original_hash
        );
    }
    for index in 1..=2 {
        let mut explicit = request(&url_a, &destination, "Ignored title");
        explicit.output.automatic = false;
        explicit.output_path = Some(
            destination
                .join("Explicit 100% 空间.mp4")
                .display()
                .to_string(),
        );
        let path = published(download(&tools, explicit));
        assert_eq!(
            path.file_stem().unwrap(),
            if index == 1 {
                "Explicit 100% 空间"
            } else {
                "Explicit 100% 空间 (2)"
            }
        );
    }
    for title in ["Same title", "Sidecar only"] {
        if title == "Sidecar only" {
            fs::write(
                destination.join("Sidecar only.en.srt"),
                b"existing subtitles",
            )
            .unwrap();
        }
        let before = server.media_get_count();
        let mut skip = request(&url_a, &destination, title);
        if title == "Sidecar only" {
            skip.write_subtitles = true;
            skip.subtitle_lang = Some("en".into());
        }
        skip.output.preferences.existing = ExistingFilePolicy::Skip;
        let result = download(&tools, skip);
        assert_eq!(result.disposition, OutputDisposition::SkippedExisting);
        assert!(result.result.is_ok());
        assert_eq!(
            server.media_get_count(),
            before,
            "Skip sent a media request"
        );
    }
    for mode in [
        OutputNamingMode::Title,
        OutputNamingMode::TitleId,
        OutputNamingMode::UploaderTitleId,
        OutputNamingMode::Custom,
    ] {
        let mut named = request(&url_a, &destination, "Named 100% 空间.mp4");
        named.output.preferences.naming = mode;
        named.output.preferences.template = "{{{title}}} [{id}] 100%".into();
        let expected = named
            .output
            .preferences
            .basename(&named.output.context)
            .unwrap();
        assert_eq!(
            published(download(&tools, named)).file_stem().unwrap(),
            expected.as_str()
        );
    }
    let mut sections = request(&url_a, &destination, "Sections");
    sections.download_section_args = vec![
        "*00:00:00-00:00:00.3".into(),
        "*00:00:00.3-00:00:00.6".into(),
    ];
    let mut section_settings = tools_transcode_settings();
    section_settings.container_policy = crate::infrastructure::ContainerPolicy::Mkv;
    sections.output.transcode = Some(section_settings);
    published(download(&tools, sections.clone()));
    assert!(destination.join("Sections - 01.mkv").is_file());
    assert!(destination.join("Sections - 02.mkv").is_file());
    assert!(!destination.join("Sections - 01.mp4").exists());
    sections.output.preferences.existing = ExistingFilePolicy::Skip;
    let before = server.media_get_count();
    assert_eq!(
        download(&tools, sections).disposition,
        OutputDisposition::SkippedExisting
    );
    assert_eq!(server.media_get_count(), before);
    let mut transcode = request(&url_a, &destination, "Transcoded");
    let mut settings = tools_transcode_settings();
    settings.container_policy = crate::infrastructure::ContainerPolicy::Mkv;
    transcode.output.transcode = Some(settings);
    let converted = published(download(&tools, transcode));
    assert_eq!(converted.extension().unwrap(), "mkv");
    assert_eq!(fs::read(&video).unwrap(), fs::read(&original).unwrap());
    for (title, source, format) in [
        (
            "Copied audio",
            Some(audio.as_path()),
            MusicDownloadFormat::Wav,
        ),
        (
            "Converted audio",
            Some(audio.as_path()),
            MusicDownloadFormat::Flac,
        ),
        ("Online audio", None, MusicDownloadFormat::Mp3),
    ] {
        let first = music(
            &tools,
            source,
            &format!("{}/audio.wav", server.address),
            &destination,
            title,
            format,
            false,
        );
        assert_eq!(first.0, OutputDisposition::Published, "{:?}", first.1);
        let first = PathBuf::from(first.1.unwrap());
        let before = crate::infrastructure::sha256_hex_file(&first).unwrap();
        let next = music(
            &tools,
            source,
            &format!("{}/audio.wav", server.address),
            &destination,
            title,
            format,
            false,
        );
        assert_eq!(next.0, OutputDisposition::Published, "{:?}", next.1);
        assert_ne!(PathBuf::from(next.1.unwrap()), first);
        assert_eq!(
            crate::infrastructure::sha256_hex_file(&first).unwrap(),
            before
        );
    }
    let (tx, rx) = mpsc::channel();
    let legacy = MusicDownloadJob {
        output: OutputRequest {
            literal_name: "Legacy exact.wav".into(),
            preferences: OutputPreferences {
                naming: OutputNamingMode::TitleId,
                ..Default::default()
            },
            ..Default::default()
        },
        item_id: 1,
        workflow_id: 1,
        source_url: format!("{}/audio.wav", server.address),
        title: "Different metadata title".into(),
        album_title: String::new(),
        output_dir: destination.clone(),
        choice: MusicDownloadChoice {
            mode: MusicDownloadMode::Unified,
            unified_format: MusicDownloadFormat::Wav,
            embed_cover: false,
            write_tags: false,
            ..Default::default()
        },
        source_acodec: "pcm_s16le".into(),
        cache_media_path: Some(audio.clone()),
        cover_path: None,
        cover_cache_dir: None,
        thumbnail_url: String::new(),
        use_cookies: false,
    };
    run_music_download_worker(
        tools.clone(),
        legacy,
        tx,
        Arc::new(Mutex::new(None)),
        Arc::new(AtomicBool::new(false)),
    );
    let legacy = rx
        .into_iter()
        .find_map(|event| {
            if let MusicDownloadEvent::Finished { result, .. } = event {
                Some(PathBuf::from(result.unwrap()))
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(legacy.file_name().unwrap(), "Legacy exact.wav");
    assert_eq!(fs::read(legacy).unwrap(), fs::read(&audio).unwrap());
    // Real yt-dlp subtitles, chapter sidecars and split chapters from controlled
    // extractor metadata. The media bytes still come from the local HTTP server.
    let info = root.join("fixture.info.json");
    let metadata = serde_json::json!({ "id": "fixture", "title": "Chapter source", "extractor": "generic", "extractor_key": "Generic", "webpage_url": url_a, "url": url_a, "ext": "mp4", "duration": 0.6, "formats": [{"format_id":"test","url":url_a,"ext":"mp4","vcodec":"mpeg4","acodec":"aac"}], "subtitles": {"en":[{"ext":"vtt","url":format!("{}/captions.vtt",server.address)}]}, "chapters": [{"title":"Intro","start_time":0.0,"end_time":0.3},{"title":"End","start_time":0.3,"end_time":0.6}] });
    fs::write(&info, serde_json::to_vec(&metadata).unwrap()).unwrap();
    let config = local_tool.parent().unwrap().join("yt-dlp.conf");
    fs::write(&config, format!("--load-info-json '{}'\n", info.display())).unwrap();
    let configured = tools.clone();
    let mut sidecars = request(&url_a, &destination, "With sidecars");
    sidecars.write_subtitles = true;
    sidecars.subtitle_lang = Some("en".into());
    sidecars.write_chapters = true;
    let sidecar_media = published(download(&configured, sidecars.clone()));
    assert!(sidecar_media.with_extension("en.srt").is_file());
    assert!(sidecar_media.with_extension("chapters.xml").is_file());
    let sidecar_media2 = published(download(&configured, sidecars));
    assert!(sidecar_media2.with_extension("en.srt").is_file());
    let mut subtitles = request(&url_a, &destination, "Standalone subtitles");
    subtitles.target_kind = DownloadTargetKind::Subtitle;
    subtitles.subtitle_lang = Some("en".into());
    subtitles.write_subtitles = true;
    subtitles.output_path = Some(
        destination
            .join("Standalone 100% 空间.srt")
            .display()
            .to_string(),
    );
    let standalone = published(download(&configured, subtitles.clone()));
    assert_eq!(standalone.extension().unwrap(), "srt");
    assert!(
        fs::read_to_string(&standalone)
            .unwrap()
            .contains("Example subtitle")
    );
    subtitles.output.preferences.existing = ExistingFilePolicy::Skip;
    let before = fs::read_to_string(root.join("http-requests.jsonl"))
        .unwrap()
        .lines()
        .count();
    assert_eq!(
        download(&configured, subtitles).disposition,
        OutputDisposition::SkippedExisting
    );
    assert_eq!(
        fs::read_to_string(root.join("http-requests.jsonl"))
            .unwrap()
            .lines()
            .count(),
        before
    );
    let chapters = music(
        &configured,
        None,
        &url_a,
        &destination,
        "Chapter export",
        MusicDownloadFormat::Mp3,
        true,
    );
    assert_eq!(chapters.0, OutputDisposition::Published, "{:?}", chapters.1);
    assert!(chapters.2);
    assert!(Path::new(&chapters.1.unwrap()).is_dir());
    fs::remove_file(&config).unwrap();
    let mut external = tools.clone();
    let external_file = root.join("external.conf");
    fs::write(
        &external_file,
        "--force-overwrites\n-o 'contained/%(title)s [%(id)s].%(ext)s'\n",
    )
    .unwrap();
    external.yt_dlp_config = external_file.display().to_string();
    let path = published(download(
        &external,
        request(&url_a, &destination, "External name"),
    ));
    assert_eq!(
        path.parent().unwrap(),
        destination.join("contained").canonicalize().unwrap()
    );
    assert!(
        path.file_stem()
            .unwrap()
            .to_string_lossy()
            .starts_with("External name [")
    );
    let before = server.media_get_count();
    fs::write(&external_file, "--exec 'echo bypass'\n").unwrap();
    let failed = download(&external, request(&url_a, &destination, "Unsafe config"));
    assert!(failed.result.unwrap_err().contains("--exec"));
    assert_eq!(server.media_get_count(), before);

    // Conversion of user-supplied media stages a copy and preserves its input.
    let before_input = crate::infrastructure::sha256_hex_file(&video).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut settings = tools_transcode_settings();
    settings.container_policy = crate::infrastructure::ContainerPolicy::Mkv;
    run_builtin_transcode_worker(
        tools.clone(),
        settings,
        video.display().to_string(),
        root.join("unused-legacy-temp"),
        1,
        1,
        tx,
        Arc::new(Mutex::new(None)),
        Arc::new(AtomicBool::new(false)),
        OutputRequest::default(),
    );
    let result = rx
        .into_iter()
        .find_map(|e| {
            if let PostProcessEvent::Finished(result) = e {
                Some(result)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(result.disposition, OutputDisposition::Published);
    assert!(Path::new(&result.result.unwrap()).is_file());
    assert_eq!(
        crate::infrastructure::sha256_hex_file(&video).unwrap(),
        before_input
    );
    let mut config =
        AppConfig::default().with_fixture_file(crate::infrastructure::runtime_config_file_path());
    config.download_dir = destination.display().to_string();
    config.yt_dlp_path = tools.yt_dlp.clone();
    config.ffmpeg_path = tools.ffmpeg.clone();
    config.thumbnail_mode = crate::infrastructure::PostProcessMode::Off;
    config.use_browser_cookies = false;
    config.save().unwrap();
    // The existing cookie host parser excludes loopback names. Route a reserved
    // .invalid test name through the counted localhost HTTP proxy instead.
    // No external service or browser account is involved.
    config.proxy_enabled = true;
    config.proxy_url = server.address.clone();
    config.save().unwrap();
    let native = serde_json::json!({"url":"http://output-fixture.invalid/a.mp4","videoFormatId":"best","videoIsMuxed":true,"fileName":"Native literal.mp4","title":"Ignored", "thumbnailMode":"off","subtitleMode":"off","chapterMode":"off","useAria2":false});
    for index in 1..=2 {
        let result =
            crate::infrastructure::browser_native_messaging::output_review_native_download(&native)
                .unwrap();
        assert_eq!(result["outcome"], "Published");
        let path = PathBuf::from(result["outputPath"].as_str().unwrap());
        assert!(path.is_file());
        assert_eq!(
            path.file_stem().unwrap(),
            if index == 1 {
                "Native literal"
            } else {
                "Native literal (2)"
            }
        );
    }
    let mut updated = config.clone();
    updated.output_existing_file = ExistingFilePolicy::Skip;
    updated.save().unwrap();
    let before = server.media_get_count();
    assert_eq!(
        crate::infrastructure::browser_native_messaging::output_review_native_download(&native)
            .unwrap()["outcome"],
        "SkippedExisting"
    );
    assert_eq!(server.media_get_count(), before);
    assert_eq!(
        crate::infrastructure::sha256_hex_file(&original).unwrap(),
        original_hash
    );
    storage::clear_disposable(storage::paths(), &[]).unwrap();
    assert!(original.is_file());
    println!(
        "{mode:?}: real yt-dlp repeat/different URL, explicit percent/CJK, zero-GET skips, real FFmpeg built-in conversion, cached copy/conversion, online audio, subtitle/chapter groups, external config and native final paths passed; media GETs={}",
        server.media_get_count()
    );
}

fn tools_transcode_settings() -> crate::infrastructure::TranscodeIntentSettings {
    crate::infrastructure::TranscodeIntentSettings::default()
}
