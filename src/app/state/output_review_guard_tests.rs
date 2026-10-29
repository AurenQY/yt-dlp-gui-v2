//! R-OUT-016..019: real configured tools, protected input and metadata intent.
use super::review_round1::metadata;
use super::*;

fn json(path: &Path, value: serde_json::Value) {
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

fn counts(path: &Path) -> [usize; 3] {
    crate::infrastructure::output_config_observation_counts(path)
}

fn difference(after: [usize; 3], before: [usize; 3]) -> [usize; 3] {
    std::array::from_fn(|i| after[i] - before[i])
}

fn configured(
    tools: &ToolPaths,
    folder: &Path,
    explicit: bool,
    content: &str,
) -> (ToolPaths, PathBuf) {
    let config = if explicit {
        folder.join("external.conf")
    } else {
        Path::new(&tools.yt_dlp)
            .parent()
            .unwrap()
            .join("yt-dlp.conf")
    };
    fs::write(&config, content).unwrap();
    fs::write(folder.join("config-before.conf"), content).unwrap();
    let mut selected = tools.clone();
    selected.yt_dlp_config = if explicit {
        config.display().to_string()
    } else {
        String::new()
    };
    (selected, config)
}

fn rejected(
    tools: &ToolPaths,
    config: &Path,
    folder: &Path,
    url: &str,
    server: &Server,
    expected: &str,
    validations: usize,
) {
    let original = fs::read(folder.join("sentinel.txt")).unwrap();
    let default_before =
        fs::read(crate::infrastructure::runtime_paths::application_root().join("existing.mp4"))
            .unwrap();
    let requests_before = server.media_get_count();
    let mut routes = Vec::new();
    for route in [
        "direct-analysis",
        "known-analysis",
        "unknown-probe",
        "automatic-preflight",
        "playlist-discovery",
        "music-flat",
        "music-formats",
    ] {
        let before = counts(config);
        let result = match route {
            "direct-analysis" => tools
                .analyze_url_detailed(url, false)
                .map(|_| ())
                .map_err(|e| e.message),
            "known-analysis" => tools
                .analyze_queue_item_detailed(url, false, None)
                .map(|_| ())
                .map_err(|e| e.message),
            "unknown-probe" => tools
                .prepare_source_probe_command(&url.replace("known.mp4", "unknown.mp4"), false)
                .map(|_| ()),
            "automatic-preflight" => {
                let mut req = request(url, folder, "Guard output");
                req.output.context.resolved = false;
                tools.prepare_download(&req).map(|_| ())
            }
            "playlist-discovery" => tools
                .prepare_playlist_discovery_command(url, false, Some(2))
                .map(|_| ()),
            "music-flat" => tools
                .prepare_music_flat_update_command(url, false)
                .map(|_| ()),
            "music-formats" => tools
                .analyze_music_output(url, false, "best", &AtomicBool::new(false))
                .map(|_| ()),
            _ => unreachable!(),
        };
        let error = result.unwrap_err();
        assert!(error.contains(expected), "{route}: {error}");
        assert_eq!(
            difference(counts(config), before),
            [1, 1, validations],
            "{route}"
        );
        routes.push(serde_json::json!({"route":route,"error":error,"application_counts":difference(counts(config), before)}));
    }
    assert_eq!(server.media_get_count(), requests_before);
    assert_eq!(fs::read(folder.join("sentinel.txt")).unwrap(), original);
    assert_eq!(
        fs::read(crate::infrastructure::runtime_paths::application_root().join("existing.mp4"))
            .unwrap(),
        default_before
    );
    assert_eq!(
        fs::read(config).unwrap(),
        fs::read(folder.join("config-before.conf")).unwrap()
    );
    json(
        &folder.join("case-result.json"),
        serde_json::json!({"kind":"rejected","routes":routes,
        "media_gets_before":requests_before,"media_gets_after":server.media_get_count(),
        "sentinel_unchanged":true,"default_output_unchanged":true,"config_unchanged":true}),
    );
}

fn metadata_and_output(
    root: &Path,
    tools: &ToolPaths,
    video: &Path,
    server: &Server,
    explicit: bool,
    bom: bool,
) {
    let label = format!(
        "guard-{}-benign-{}",
        if explicit { "explicit" } else { "portable" },
        if bom { "bom" } else { "utf8" }
    );
    let folder = root.join(&label);
    fs::create_dir_all(&folder).unwrap();
    let url = format!("{}/known.mp4?{label}", server.address);
    metadata(tools, root, &url, "Guard source", false);
    let content = format!(
        "{}--user-agent Guard\n--load-info-json \"{}\"\n--no-simulate\n--force-overwrites\n-o existing.mp4\n",
        if bom { "\u{feff}" } else { "" },
        root.join("round1-input.json").display()
    );
    let (selected, config) = configured(tools, &folder, explicit, &content);
    let old = folder.join("existing.mp4");
    fs::write(&old, b"established output sentinel").unwrap();
    let original = fs::read(&old).unwrap();
    let default = crate::infrastructure::runtime_paths::application_root().join("existing.mp4");
    let default_before = fs::read(&default).unwrap();
    let media_before = server.media_get_count();
    let mut phases = Vec::new();
    let before = counts(&config);
    let analyzed = selected
        .analyze_queue_item_detailed(&url, false, None)
        .unwrap_or_else(|e| panic!("{}", e.message));
    assert!(
        analyzed.command_line.contains("--simulate")
            && analyzed.command_line.contains("--skip-download")
    );
    assert_eq!(analyzed.json["title"], "Guard source");
    assert_eq!(difference(counts(&config), before), [1, 1, 1]);
    json(
        &folder.join("known-analysis.json"),
        serde_json::json!({"command":analyzed.command_line,"metadata":analyzed.json}),
    );
    phases.push(serde_json::json!({"phase":"known-analysis","application_counts":difference(counts(&config),before)}));
    let mut req = request(&url, &folder, "Unresolved title");
    req.output.context.resolved = false;
    let before = counts(&config);
    let prepared = selected.prepare_download(&req).unwrap();
    assert_eq!(server.media_get_count(), media_before);
    assert_eq!(fs::read(&old).unwrap(), original);
    assert_eq!(fs::read(&default).unwrap(), default_before);
    let work = prepared
        .output_transaction
        .as_ref()
        .unwrap()
        .lock()
        .unwrap()
        .work
        .clone();
    assert!(
        work.canonicalize().unwrap().starts_with(
            storage::require_ready()
                .unwrap()
                .cache_root
                .canonicalize()
                .unwrap()
        )
    );
    json(
        &folder.join("prepared-before-media.json"),
        serde_json::json!({"command":prepared.command_line,
        "work":work,"output":prepared.output_path,"media_gets":server.media_get_count(),
        "existing_unchanged":true,"default_output_unchanged":true,"application_counts":difference(counts(&config),before)}),
    );
    assert_eq!(difference(counts(&config), before), [1, 1, 1]);
    drop(prepared);
    phases.push(serde_json::json!({"phase":"unresolved-preparation","application_counts":difference(counts(&config),before)}));
    let before = counts(&config);
    let result = download(&selected, req.clone());
    assert_eq!(
        result.disposition,
        OutputDisposition::Published,
        "{:?}",
        result.result
    );
    let output = PathBuf::from(result.result.unwrap());
    assert_eq!(output.file_name().unwrap(), "existing (2).mp4");
    assert_eq!(
        crate::infrastructure::sha256_hex_file(&output).unwrap(),
        crate::infrastructure::sha256_hex_file(video).unwrap()
    );
    assert_eq!(server.media_get_count(), media_before + 1);
    assert_eq!(difference(counts(&config), before), [1, 1, 1]);
    phases.push(serde_json::json!({"phase":"unresolved-download","application_counts":difference(counts(&config),before)}));
    req.output.preferences.existing = ExistingFilePolicy::Skip;
    let before = counts(&config);
    let skipped = download(&selected, req);
    assert_eq!(
        skipped.disposition,
        OutputDisposition::SkippedExisting,
        "{:?}",
        skipped.result
    );
    assert_eq!(difference(counts(&config), before), [1, 1, 1]);
    phases.push(
        serde_json::json!({"phase":"skip","application_counts":difference(counts(&config),before)}),
    );
    assert_eq!(server.media_get_count(), media_before + 1);
    assert_eq!(fs::read(&old).unwrap(), original);
    assert_eq!(fs::read(&default).unwrap(), default_before);
    assert_eq!(fs::read(&config).unwrap(), content.as_bytes());
    json(
        &folder.join("case-result.json"),
        serde_json::json!({"kind":"metadata-output","explicit":explicit,"bom":bom,
        "phases":phases,"output":output,"source_sha256":crate::infrastructure::sha256_hex_file(video).unwrap(),
        "media_gets_before":media_before,"media_gets_after":server.media_get_count(),
        "existing_unchanged":true,"default_output_unchanged":true,"config_unchanged":true}),
    );
}

fn metadata_routes(root: &Path, tools: &ToolPaths, server: &Server, explicit: bool) {
    let folder = root.join(format!(
        "guard-{}-sleep",
        if explicit { "explicit" } else { "portable" }
    ));
    fs::create_dir_all(&folder).unwrap();
    let url = format!("{}/unknown.mp4", server.address);
    metadata(tools, root, &url, "Sleep control", false);
    let content = format!(
        "-t sleep\n--user-agent \"--\"\n--no-simulate\n--force-overwrites\n--load-info-json \"{}\"\n",
        root.join("round1-input.json").display()
    );
    let (selected, config) = configured(tools, &folder, explicit, &content);
    let before_media = server.media_get_count();
    let mut observations = Vec::new();
    for route in [
        "unknown-probe",
        "playlist-discovery",
        "music-flat",
        "music-formats",
    ] {
        let before = counts(&config);
        let command_line = if route == "music-formats" {
            selected
                .analyze_music_output(&url, false, "best", &AtomicBool::new(false))
                .unwrap()
                .command_line
        } else {
            let mut command = match route {
                "unknown-probe" => {
                    selected
                        .prepare_source_probe_command(&url, false)
                        .unwrap()
                        .0
                }
                "playlist-discovery" => selected
                    .prepare_playlist_discovery_command(&url, false, Some(2))
                    .unwrap(),
                _ => selected
                    .prepare_music_flat_update_command(&url, false)
                    .unwrap(),
            };
            let args = command
                .get_args()
                .map(|v| v.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            assert!(
                args.iter().any(|v| v == "--simulate")
                    && args.iter().any(|v| v == "--skip-download")
            );
            let result = command.output().unwrap();
            fs::write(folder.join(format!("{route}.stdout.log")), &result.stdout).unwrap();
            fs::write(folder.join(format!("{route}.stderr.log")), &result.stderr).unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            assert!(String::from_utf8_lossy(&result.stdout).contains("Sleep control"));
            args.join(" ")
        };
        assert_eq!(difference(counts(&config), before), [1, 1, 1]);
        observations.push(serde_json::json!({"route":route,"command":command_line,"application_counts":difference(counts(&config),before)}));
    }
    assert_eq!(server.media_get_count(), before_media);
    json(
        &folder.join("case-result.json"),
        serde_json::json!({"kind":"metadata-routes","observations":observations,
        "media_gets_before":before_media,"media_gets_after":server.media_get_count()}),
    );
}

fn preset_controls(root: &Path, tools: &ToolPaths, server: &Server) {
    let _owner = storage::use_cache().unwrap();
    let url = format!("{}/known.mp4", server.address);
    metadata(tools, root, &url, "Preset control", false);
    for preset in ["mp3", "aac", "mp4", "mkv"] {
        let folder = storage::cache_directory("yt-dlp-temp")
            .unwrap()
            .join(format!("guard-preset-{preset}"));
        fs::create_dir_all(&folder).unwrap();
        let before = server.media_get_count();
        let result = Command::new(&tools.yt_dlp)
            .args([
                "--ignore-config",
                "-t",
                preset,
                "--no-simulate",
                "--force-overwrites",
                "--ffmpeg-location",
            ])
            .arg(&tools.ffmpeg)
            .arg("--cache-dir")
            .arg(storage::cache_directory("yt-dlp").unwrap())
            .args([
                "-P",
                &format!("home:{}", folder.display()),
                "-P",
                &format!("temp:{}", folder.display()),
                "-o",
                "preset.%(ext)s",
                "--load-info-json",
            ])
            .arg(root.join("round1-input.json"))
            .arg(&url)
            .output()
            .unwrap();
        fs::write(folder.join("stdout.log"), &result.stdout).unwrap();
        fs::write(folder.join("stderr.log"), &result.stderr).unwrap();
        assert!(
            result.status.success(),
            "{preset}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let files = fs::read_dir(&folder)
            .unwrap()
            .map(|v| v.unwrap().path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("preset.")
            })
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 1, "{preset}: {files:?}");
        let probe = Command::new(
            Path::new(&tools.ffmpeg)
                .parent()
                .unwrap()
                .join("ffprobe.exe"),
        )
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_name,codec_type",
            "-of",
            "json",
        ])
        .arg(&files[0])
        .output()
        .unwrap();
        assert!(probe.status.success());
        let probed: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
        if preset == "mp3" || preset == "aac" {
            assert_eq!(probed["streams"].as_array().unwrap().len(), 1);
            assert_eq!(probed["streams"][0]["codec_name"], preset);
        }
        if preset == "mkv" {
            assert_eq!(files[0].extension().unwrap(), "mkv");
        }
        assert_eq!(server.media_get_count(), before + 1);
        json(
            &folder.join("case-result.json"),
            serde_json::json!({"kind":"raw-tool-preset-control","preset":preset,
            "output":files[0],"probe":probed,"media_gets_before":before,"media_gets_after":server.media_get_count()}),
        );
    }
}

pub(super) fn run(root: &Path, tools: &ToolPaths, video: &Path, server: &Server) {
    // Use a tiny explicit routing dataset to exercise the production known-single
    // branch and unknown probe separately; metadata comes from real info-file input.
    let resources = crate::infrastructure::runtime_paths::application_root().join("resources");
    fs::create_dir_all(&resources).unwrap();
    json(
        &resources.join("extractor-index.json"),
        serde_json::json!({"y":[["GuardSingle",r"http://127\.0\.0\.1:[0-9]+/known\.mp4"],["GuardUnknown",r"http://127\.0\.0\.1:"]],"ys":[[0,1]],"yd":0,"g":[["Never",r"(?!)"]]}),
    );
    assert_eq!(
        crate::infrastructure::extractor_index::matching_yt_source_scope(&format!(
            "{}/known.mp4",
            server.address
        ))
        .unwrap(),
        crate::infrastructure::extractor_index::YtSourceScope::Single
    );
    assert_eq!(
        crate::infrastructure::extractor_index::matching_yt_source_scope(&format!(
            "{}/unknown.mp4",
            server.address
        ))
        .unwrap(),
        crate::infrastructure::extractor_index::YtSourceScope::Unknown
    );
    fs::write(
        crate::infrastructure::runtime_paths::application_root().join("existing.mp4"),
        b"working-directory sentinel",
    )
    .unwrap();
    preset_controls(root, tools, server);
    for explicit in [false, true] {
        for bom in [false, true] {
            metadata_and_output(root, tools, video, server, explicit, bom);
        }
        metadata_routes(root, tools, server, explicit);
        for (label, content, expected, validations) in [
            (
                "hook",
                "--exec 'before_dl:cmd /c echo hook-ran>\"GUARD_SENTINEL\"'\n".to_owned(),
                "--exec",
                1,
            ),
            (
                "bom-hook",
                "\u{feff}--exec 'before_dl:cmd /c echo hook-ran>\"GUARD_SENTINEL\"'\n".to_owned(),
                "--exec",
                1,
            ),
            (
                "terminator",
                "--ignore-errors\n--\n".to_owned(),
                "terminator: --",
                0,
            ),
            (
                "overwrite-terminator",
                "--ignore-errors\n--force-overwrites\n--\n".to_owned(),
                "terminator: --",
                0,
            ),
            ("mp3-short", "-t mp3\n".to_owned(), "Unsupported preset", 1),
            (
                "mp3-long",
                "--preset-alias mp3\n".to_owned(),
                "Unsupported preset",
                1,
            ),
            (
                "mp3-attached",
                "-tmp3\n".to_owned(),
                "Unsupported preset",
                1,
            ),
            (
                "aac",
                "--preset-alias=aac\n".to_owned(),
                "Unsupported preset",
                1,
            ),
            (
                "mp4",
                "--preset-a mp4\n".to_owned(),
                "Unsupported preset",
                1,
            ),
            ("mkv", "-t mkv\n".to_owned(), "Unsupported preset", 1),
            (
                "unknown-preset",
                "-t unknown\n".to_owned(),
                "Unsupported preset",
                1,
            ),
            (
                "absolute",
                format!(
                    "--no-simulate\n--force-overwrites\n-o \"{}\"\n",
                    root.join("outside.mp4").display()
                ),
                "Absolute",
                1,
            ),
        ] {
            let folder = root.join(format!(
                "guard-{}-{label}",
                if explicit { "explicit" } else { "portable" }
            ));
            fs::create_dir_all(&folder).unwrap();
            fs::write(folder.join("sentinel.txt"), b"sentinel preserved").unwrap();
            let content = content.replace(
                "GUARD_SENTINEL",
                &folder.join("sentinel.txt").display().to_string(),
            );
            let (selected, config) = configured(tools, &folder, explicit, &content);
            rejected(
                &selected,
                &config,
                &folder,
                &format!("{}/known.mp4", server.address),
                server,
                expected,
                validations,
            );
        }
    }
    println!(
        "R-OUT-016..019 protected-config acceptance passed: {}",
        root.display()
    );
}
