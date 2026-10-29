//! R-OUT-001..004: production dispatch and real localhost media, both cache modes.
use super::*;

pub(super) fn run(root: &Path, tools: &ToolPaths, audio: &Path, video: &Path, server: &Server) {
    conversion(root, tools, video);
    direct_dispatch(root, tools, server);
    audio_dispatch(root, tools, audio, server);
    println!(
        "R-OUT-001..004 production-entry regressions passed: {}",
        root.display()
    );
}

pub(super) fn new_state(root: &Path, tools: &ToolPaths, name: &str) -> AppState {
    let mut config = AppConfig::default().with_fixture_file(root.join(format!("{name}.yaml")));
    config.download_dir = root.join(name).display().to_string();
    config.yt_dlp_path = tools.yt_dlp.clone();
    config.ffmpeg_path = tools.ffmpeg.clone();
    config.use_browser_cookies = false;
    config.thumbnail_mode = crate::infrastructure::PostProcessMode::Off;
    let mut state = AppState::from_runtime(config, tools.clone());
    // All cases share one isolated application root, whose previous queue is
    // deliberately persisted by production code. Each case starts a new queue.
    state.queue_items.clear();
    // Repeated AppState construction is specific to this fixture. Let each
    // normal two-second startup cleanup finish before the next user action.
    // No artificial cache lease is held across the download under test.
    thread::sleep(Duration::from_millis(2100));
    let started = Instant::now();
    loop {
        if let Ok(guard) = storage::use_cache() {
            drop(guard);
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(20));
    }
    state
}

pub(super) fn finished(state: &AppState) -> DownloadResult {
    loop {
        match state
            .download_result_rx
            .recv_timeout(Duration::from_secs(40))
            .unwrap()
        {
            DownloadEvent::Finished(result) => return result,
            _ => {}
        }
    }
}

pub(super) fn music_finished(
    state: &AppState,
) -> (
    MusicDownloadSourceKind,
    OutputDisposition,
    Result<String, String>,
) {
    loop {
        match state
            .music_download_event_rx
            .recv_timeout(Duration::from_secs(40))
            .unwrap()
        {
            MusicDownloadEvent::Finished {
                source_kind,
                disposition,
                result,
                ..
            } => return (source_kind, disposition, result),
            _ => {}
        }
    }
}

fn convert(
    tools: &ToolPaths,
    source: &Path,
    policy: ExistingFilePolicy,
) -> crate::app::post_process_worker::PostProcessResult {
    let (tx, rx) = mpsc::channel();
    let mut settings = tools_transcode_settings();
    settings.container_policy = crate::infrastructure::ContainerPolicy::Mkv;
    run_builtin_transcode_worker(
        tools.clone(),
        settings,
        source.display().to_string(),
        source.parent().unwrap().join("legacy-unused-temp"),
        1,
        1,
        tx,
        Arc::new(Mutex::new(None)),
        Arc::new(AtomicBool::new(false)),
        OutputRequest {
            preferences: OutputPreferences {
                existing: policy,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    rx.into_iter()
        .find_map(|event| {
            if let PostProcessEvent::Finished(result) = event {
                Some(result)
            } else {
                None
            }
        })
        .unwrap()
}

fn conversion(root: &Path, tools: &ToolPaths, video: &Path) {
    let folder = root.join("round1-conversion");
    fs::create_dir_all(&folder).unwrap();
    let source = folder.join("Clip.mp4");
    fs::copy(video, &source).unwrap();
    let unrelated = folder.join("Clip.txt");
    fs::write(&unrelated, b"unrelated same-stem document").unwrap();
    let before = crate::infrastructure::sha256_hex_file(&source).unwrap();
    let result = convert(tools, &source, ExistingFilePolicy::Skip);
    assert_eq!(
        result.disposition,
        OutputDisposition::Published,
        "{:?}",
        result.result
    );
    assert_eq!(
        Path::new(&result.result.unwrap()).file_name().unwrap(),
        "Clip.mkv"
    );
    let target = folder.join("Clip.mkv");
    let target_hash = crate::infrastructure::sha256_hex_file(&target).unwrap();
    let result = convert(tools, &source, ExistingFilePolicy::Skip);
    assert_eq!(result.disposition, OutputDisposition::SkippedExisting);
    assert_eq!(
        crate::infrastructure::sha256_hex_file(&target).unwrap(),
        target_hash
    );
    assert_eq!(
        crate::infrastructure::sha256_hex_file(&source).unwrap(),
        before
    );
    assert_eq!(
        fs::read(&unrelated).unwrap(),
        b"unrelated same-stem document"
    );
    let source = folder.join("Keep.mp4");
    fs::copy(video, &source).unwrap();
    assert_eq!(
        Path::new(
            &convert(tools, &source, ExistingFilePolicy::KeepBoth)
                .result
                .unwrap()
        )
        .file_name()
        .unwrap(),
        "Keep.mkv"
    );
    println!(
        "R-OUT-001: real Skip conversion publishes an absent MKV, skips the occupied MKV, preserves source/document; KeepBoth has no false suffix"
    );
}

/// Controlled source metadata enters through the real yt-dlp info-file option.
pub(super) fn metadata(
    tools: &ToolPaths,
    root: &Path,
    url: &str,
    title: &str,
    audio: bool,
) -> serde_json::Value {
    let path = root.join("round1-input.json");
    let ext = if audio { "wav" } else { "mp4" };
    let mut value = serde_json::json!({"id":"source-known-123","title":title,"uploader":"Known uploader","upload_date":"20261026", "extractor":"generic", "extractor_key":"Generic", "webpage_url":url,"original_url":url,"url":url,"ext":ext,"format_id":"fixture", "acodec":if audio {"pcm_s16le"} else {"aac"},"vcodec":if audio {"none"} else {"mpeg4"},"duration":0.6,"protocol":"http","chapters":[{"start_time":0.0,"end_time":0.3,"title":"Intro"},{"start_time":0.3,"end_time":0.6,"title":"End"}]});
    value["formats"] = serde_json::json!([{"format_id":"fixture","url":url,"ext":ext,"acodec":value["acodec"],"vcodec":value["vcodec"],"protocol":"http"}]);
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    fs::write(
        Path::new(&tools.yt_dlp)
            .parent()
            .unwrap()
            .join("yt-dlp.conf"),
        format!("--load-info-json \"{}\"\n", path.display()),
    )
    .unwrap();
    value
}

fn direct_dispatch(root: &Path, tools: &ToolPaths, server: &Server) {
    let url = format!("{}/watch-placeholder.mp4", server.address);
    metadata(tools, root, &url, "Actual source 空间 100%", false);
    let mut state = new_state(root, tools, "round1-title");
    let mut item = state.build_queue_item_from_url(&url);
    item.metadata_state = MetadataState::Idle;
    let id = item.id;
    assert!(item.title.starts_with("Imported "));
    state.queue_items = vec![item];
    state.start_download_task_at(id, true).unwrap();
    let captured = state.last_dispatched_download.as_ref().unwrap();
    assert!(captured.output.automatic && !captured.output.context.resolved);
    assert!(captured.output.context.title.is_empty());
    // This is a live-dispatch settings change, not a modified downstream request.
    state.config.output_naming = OutputNamingMode::Custom;
    state.config.output_template = "Changed {title}".into();
    let path = published(finished(&state));
    assert_eq!(path.file_name().unwrap(), "Actual source 空间 100%.mkv");
    let old_hash = crate::infrastructure::sha256_hex_file(&path).unwrap();
    let request = state.last_dispatched_download.clone().unwrap();
    let prepared = tools.prepare_download(&request).unwrap();
    let mut retry = request;
    retry.output.transaction = prepared.output_transaction.clone();
    retry.output.context.title = "Different retry title".into();
    retry.output.context.resolved = true;
    retry.output.preferences.naming = OutputNamingMode::Custom;
    retry.output.preferences.template = "Changed {title}".into();
    let repeated = tools.prepare_download(&retry).unwrap();
    let owner = repeated
        .output_transaction
        .as_ref()
        .unwrap()
        .lock()
        .unwrap();
    assert_eq!(owner.stem, "Actual source 空间 100% (2)");
    drop(owner);
    drop(repeated);
    drop(prepared);
    drop(retry);
    assert_eq!(
        crate::infrastructure::sha256_hex_file(&path).unwrap(),
        old_hash
    );

    // Exercise the production edit handler AND the production dispatcher.
    for (policy, suffix, outcome) in [
        (
            ExistingFilePolicy::KeepBoth,
            "",
            OutputDisposition::Published,
        ),
        (
            ExistingFilePolicy::KeepBoth,
            " (2)",
            OutputDisposition::Published,
        ),
        (
            ExistingFilePolicy::Skip,
            "",
            OutputDisposition::SkippedExisting,
        ),
    ] {
        let mut state = new_state(root, tools, "round1-manual");
        state.config.output_naming = OutputNamingMode::Custom;
        state.config.output_template = "Global {title}".into();
        state.config.output_existing_file = policy;
        let mut item = state.build_queue_item_from_url(&url);
        item.metadata_state = MetadataState::Idle;
        item.selection.file_name = "Manual batch 空间 100%".into();
        let id = item.id;
        state.queue_items = vec![item];
        state.normalize_item_download_base_name(0);
        state.start_download_task_at(id, true).unwrap();
        let captured = state.last_dispatched_download.as_ref().unwrap();
        assert!(!captured.output.automatic);
        assert_eq!(captured.file_name, "Manual batch 空间 100%");
        assert_eq!(captured.output.literal_name, "Manual batch 空间 100%");
        let result = finished(&state);
        assert_eq!(result.disposition, outcome, "{:?}", result.result);
        let path = PathBuf::from(result.result.unwrap());
        assert_eq!(
            path.file_name().unwrap(),
            format!("Manual batch 空间 100%{suffix}.mkv").as_str()
        );
    }
    println!(
        "R-OUT-002/003: Idle actual dispatch resolves source title, freezes live settings/retry plan, preserves manual Unicode/percent names and KeepBoth/Skip"
    );
}

pub(super) fn cached_stream(state: &AppState, seed: &MusicStreamSeed, audio: &Path) -> PathBuf {
    let directory = state
        .music_stream_cache_root()
        .join(sanitize_music_cache_key(&seed.cache_key));
    fs::create_dir_all(&directory).unwrap();
    fs::copy(audio, directory.join(format!("audio.{}", seed.ext))).unwrap();
    let manifest = serde_json::json!({"source_url":seed.source_url,"title":seed.title,"ext":seed.ext,"format_id":seed.format_id,"acodec":seed.acodec,"audio_only":true,"complete":true,"expected_bytes":fs::metadata(audio).unwrap().len(),"updated_unix_seconds":SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()});
    fs::write(
        directory.join("manifest.yaml"),
        serde_yaml::to_string(&manifest).unwrap(),
    )
    .unwrap();
    directory
}

fn audio_dispatch(root: &Path, tools: &ToolPaths, audio: &Path, server: &Server) {
    for (rule, expected) in [
        (
            OutputNamingMode::TitleId,
            "Resolved audio 空间 100% [source-known-123]",
        ),
        (
            OutputNamingMode::UploaderTitleId,
            "Known uploader - Resolved audio 空间 100% [source-known-123]",
        ),
        (
            OutputNamingMode::Custom,
            "Resolved audio 空间 100%--source-known-123--Known uploader--20261026",
        ),
    ] {
        for route in ["online", "copy", "convert", "chapters"] {
            let name = format!("round1-audio-{rule:?}-{route}");
            let url = format!("{}/audio.wav?case={name}", server.address);
            metadata(tools, root, &url, "Resolved audio 空间 100%", true);
            let mut state = new_state(root, tools, &name);
            state.config.output_naming = rule;
            state.config.output_template = "{title}--{id}--{uploader}--{upload_date}".into();
            let flat = PlaylistEntrySeed {
                source_url: url.clone(),
                title: "Flat placeholder".into(),
                album_title: String::new(),
                thumbnail_url: String::new(),
                thumbnail_hint: String::new(),
                duration_text: String::new(),
            };
            assert!(state.append_music_compact_seed(flat));
            let id = state.queue_items[0].id;
            let seed = resolve_music_stream_seed_with_cookie_fallback(tools, &url, false)
                .result
                .unwrap();
            let cache = cached_stream(&state, &seed, audio);
            state.music.music_prefetch_pending_item_id = Some(id);
            state.music.music_prefetch_session_id = 7;
            state.apply_music_stream_result(MusicStreamResolveEvent::Resolve {
                item_id: id,
                session_id: 7,
                source: url.clone(),
                play_after_resolve: false,
                result: Ok(seed),
            });
            assert!(!state.queue_items[0].metadata_loaded());
            let context = state
                .output_request_for_item(&state.queue_items[0], false)
                .context;
            assert!(context.resolved);
            assert_eq!(
                (
                    &context.title,
                    &context.id,
                    &context.uploader,
                    &context.upload_date
                ),
                (
                    &"Resolved audio 空间 100%".to_owned(),
                    &"source-known-123".to_owned(),
                    &"Known uploader".to_owned(),
                    &"20261026".to_owned()
                )
            );
            if route == "online" {
                fs::remove_file(cache.join("audio.wav")).unwrap();
                fs::remove_file(cache.join("manifest.yaml")).unwrap();
            }
            if rule == OutputNamingMode::Custom && route == "copy" {
                // A restored cache keeps its old schema and has no transient
                // naming metadata. Dispatch must resolve, not assume absence.
                state.queue_items[0].music_naming_metadata = None;
                assert!(
                    !state
                        .output_request_for_item(&state.queue_items[0], false)
                        .context
                        .resolved
                );
            }
            let mut choice = MusicDownloadChoice {
                mode: MusicDownloadMode::Unified,
                unified_format: if route == "convert" {
                    MusicDownloadFormat::Flac
                } else {
                    MusicDownloadFormat::Wav
                },
                embed_cover: false,
                write_tags: false,
                ..Default::default()
            };
            if route == "chapters" {
                choice.save_mode = MusicDownloadSaveMode::SplitChapters;
            }
            state.start_music_download_task_at(id, choice).unwrap();
            let (source_kind, disposition, result) = music_finished(&state);
            assert_eq!(
                disposition,
                OutputDisposition::Published,
                "{name}: {result:?}"
            );
            assert_eq!(
                source_kind,
                match route {
                    "online" | "chapters" => MusicDownloadSourceKind::YtDlpDownload,
                    "convert" => MusicDownloadSourceKind::CacheConvert,
                    _ => MusicDownloadSourceKind::CacheCopy,
                }
            );
            let path = PathBuf::from(result.unwrap());
            if route == "chapters" {
                assert!(path.is_dir());
                assert_eq!(path.file_name().unwrap(), expected);
                assert_eq!(fs::read_dir(&path).unwrap().count(), 2);
            } else {
                assert_eq!(
                    path.file_name().unwrap(),
                    format!(
                        "{expected}.{}",
                        if route == "convert" { "flac" } else { "wav" }
                    )
                    .as_str()
                );
            }
        }
    }
    // Missing-field input is deliberately injected at the real stream parser,
    // not synthesized as a worker naming context. Media publication remains real.
    let url = format!("{}/audio.wav?case=missing-fields", server.address);
    let mut absent = metadata(tools, root, &url, "Tool transport title", true);
    for key in ["title", "id", "uploader", "upload_date"] {
        absent.as_object_mut().unwrap().remove(key);
    }
    let mut state = new_state(root, tools, "round1-missing-fields");
    state.config.output_naming = OutputNamingMode::Custom;
    state.config.output_template = "{title}--{id}--{uploader}--{upload_date}".into();
    let mut item = state.build_queue_item_from_url(&url);
    item.metadata_state = MetadataState::Idle;
    let id = item.id;
    state.queue_items = vec![item];
    let seed = music_stream_seed_from_json(&absent, &url).unwrap();
    cached_stream(&state, &seed, audio);
    state.music.music_prefetch_pending_item_id = Some(id);
    state.music.music_prefetch_session_id = 8;
    state.apply_music_stream_result(MusicStreamResolveEvent::Resolve {
        item_id: id,
        session_id: 8,
        source: url,
        play_after_resolve: false,
        result: Ok(seed),
    });
    let context = state
        .output_request_for_item(&state.queue_items[0], false)
        .context;
    assert!(
        context.resolved
            && context.title.is_empty()
            && context.id.is_empty()
            && context.uploader.is_empty()
            && context.upload_date.is_empty()
    );
    let identity = root.join("missing-source-identity.txt");
    fs::write(&identity, context.source.as_bytes()).unwrap();
    let source_hash = crate::infrastructure::sha256_hex_file(&identity).unwrap();
    let expected = format!(
        "Untitled--source-{}--Unknown uploader--Unknown date.wav",
        &source_hash[..12]
    );
    state
        .start_music_download_task_at(
            id,
            MusicDownloadChoice {
                mode: MusicDownloadMode::Unified,
                unified_format: MusicDownloadFormat::Wav,
                embed_cover: false,
                write_tags: false,
                ..Default::default()
            },
        )
        .unwrap();
    let (_, disposition, result) = music_finished(&state);
    assert_eq!(disposition, OutputDisposition::Published, "{result:?}");
    assert_eq!(
        Path::new(&result.unwrap()).file_name().unwrap(),
        expected.as_str()
    );
    fs::remove_file(
        Path::new(&tools.yt_dlp)
            .parent()
            .unwrap()
            .join("yt-dlp.conf"),
    )
    .unwrap();
    println!(
        "R-OUT-004: real compact resolve event and actual music dispatch, presets/all four custom fields across online/copy/convert/chapters; injected genuinely absent fields use deterministic fallbacks"
    );
}
