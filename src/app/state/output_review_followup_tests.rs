//! R-OUT-012 follow-up: actual entry preparations and real option-value semantics.
use super::review_round1::{metadata, new_state};
use super::review_round3::music_item;
use super::*;

fn write_json(path: &Path, value: serde_json::Value) {
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

fn counts(path: &Path) -> [usize; 3] {
    crate::infrastructure::output_config_observation_counts(path)
}

fn delta(after: [usize; 3], before: [usize; 3]) -> [usize; 3] {
    std::array::from_fn(|i| after[i] - before[i])
}

pub(super) fn run(root: &Path, tools: &ToolPaths, video: &Path, server: &Server) {
    let alternate = root.join("alternate.webm");
    let generated = Command::new(&tools.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(video)
        .args(["-c:v", "libvpx-vp9", "-c:a", "libopus"])
        .arg(&alternate)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    option_values(root, tools, video, server);
    stream_preparation(root, tools, server);
    println!("R-OUT-012 follow-up acceptance passed: {}", root.display());
}

fn option_values(root: &Path, tools: &ToolPaths, video: &Path, server: &Server) {
    for (label, option, value, title) in [
        ("match-format", "--match-title", "-f", "Clip -f sample"),
        ("match-path", "--match-title", "-Pfoo", "Clip -Pfoo sample"),
        (
            "match-abbreviation",
            "--match-t",
            "-f",
            "Clip -f abbreviation",
        ),
        (
            "reject-format",
            "--reject-title",
            "-f",
            "Regex independent clip",
        ),
        ("agent-format", "--user-agent", "-f", "Agent clip"),
        ("referer-path", "--referer", "-Pfoo", "Referer clip"),
    ] {
        let folder = root.join(format!("followup-value-{label}"));
        fs::create_dir_all(&folder).unwrap();
        let url = format!("{}/watch.mp4?{label}", server.address);
        let mut info = metadata(tools, root, &url, title, false);
        for field in [
            "url",
            "ext",
            "format_id",
            "acodec",
            "vcodec",
            "protocol",
            "chapters",
        ] {
            info.as_object_mut().unwrap().remove(field);
        }
        info["formats"] = serde_json::json!([
            {"format_id":"app-mp4","url":format!("{}/selected.mp4",server.address),"ext":"mp4","vcodec":"mpeg4","acodec":"aac","protocol":"http","quality":0,"width":64,"height":64},
            {"format_id":"default-webm","url":format!("{}/alternate.webm",server.address),"ext":"webm","vcodec":"vp9","acodec":"opus","protocol":"http","quality":10,"width":64,"height":64}
        ]);
        let input = folder.join("input.json");
        write_json(&input, info);
        let config = folder.join("external.conf");
        fs::write(
            &config,
            format!(
                "--load-info-json \"{}\"\n{option} \"{value}\"\n",
                input.display()
            ),
        )
        .unwrap();
        let mut selected = tools.clone();
        selected.yt_dlp_config = config.display().to_string();
        let before = counts(&config);
        let media_before = server.media_get_count();
        let old = folder.join(format!("{title}.mp4"));
        fs::write(&old, b"established final sentinel").unwrap();
        let mut req = request(&url, &folder, title);
        req.format_selector = "app-mp4".into();
        req.output.context.resolved = false;
        let result = download(&selected, req);
        assert_eq!(
            result.disposition,
            OutputDisposition::Published,
            "{label}: {:?}",
            result.result
        );
        let output = PathBuf::from(result.result.unwrap());
        assert_eq!(
            output.parent().unwrap().canonicalize().unwrap(),
            folder.canonicalize().unwrap()
        );
        assert_eq!(
            output.file_name().unwrap(),
            format!("{title} (2).mp4").as_str()
        );
        assert_eq!(
            crate::infrastructure::sha256_hex_file(&output).unwrap(),
            crate::infrastructure::sha256_hex_file(video).unwrap()
        );
        assert_eq!(fs::read(&old).unwrap(), b"established final sentinel");
        assert!(!root.join("app/foo").exists());
        assert_eq!(delta(counts(&config), before), [1, 1, 1]);
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
            "stream=codec_name",
            "-of",
            "json",
        ])
        .arg(&output)
        .output()
        .unwrap();
        assert!(probe.status.success());
        let probed: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
        assert_eq!(probed["streams"][0]["codec_name"], "mpeg4");
        write_json(&folder.join("probe.json"), probed);
        // Prove the retained alternative is the tool's actual default and the
        // quoted regex/header is accepted, independently of app-added --format.
        let control = Command::new(&tools.yt_dlp)
            .arg("--config-location")
            .arg(&config)
            .args(["--skip-download", "--print", "%(format_id)s", &url])
            .output()
            .unwrap();
        fs::write(folder.join("default-control.stdout.log"), &control.stdout).unwrap();
        fs::write(folder.join("default-control.stderr.log"), &control.stderr).unwrap();
        assert!(control.status.success());
        assert!(String::from_utf8_lossy(&control.stdout).contains("default-webm"));
        write_json(
            &folder.join("case-result.json"),
            serde_json::json!({"case":label,"option":option,"value":value,
            "output":output,"application_counts":delta(counts(&config),before),"codec":"mpeg4",
            "tool_default":"default-webm","source_sha256":crate::infrastructure::sha256_hex_file(video).unwrap(),
            "media_gets_before":media_before,"media_gets_after":server.media_get_count()}),
        );
    }
}

fn stream_preparation(root: &Path, tools: &ToolPaths, server: &Server) {
    for route in ["playback", "prefetch"] {
        for explicit in [false, true] {
            let name = format!(
                "followup-{route}-{}",
                if explicit { "explicit" } else { "portable" }
            );
            let folder = root.join(&name);
            fs::create_dir_all(&folder).unwrap();
            let url = format!("{}/audio.wav?{name}", server.address);
            metadata(tools, root, &url, "Stream clip", true);
            let portable = Path::new(&tools.yt_dlp)
                .parent()
                .unwrap()
                .join("yt-dlp.conf");
            let config = if explicit {
                let path = folder.join("external.conf");
                fs::copy(&portable, &path).unwrap();
                path
            } else {
                portable
            };
            fs::write(
                folder.join("config-before.conf"),
                fs::read(&config).unwrap(),
            )
            .unwrap();
            let cookie = folder.join("empty-cookies.txt");
            fs::write(&cookie, b"# Netscape HTTP Cookie File\n").unwrap();
            let mut selected = tools.clone();
            selected.yt_dlp_config = if explicit {
                config.display().to_string()
            } else {
                String::new()
            };
            selected.browser_cookie_source = "file".into();
            selected.browser_cookie_file = cookie.display().to_string();
            let mut state = new_state(root, &selected, &name);
            let id = music_item(&mut state, &url, "Stream clip");
            let item = state.queue_item_mut_by_id(id).unwrap();
            item.selection.use_cookies = true;
            item.music_stream_url = url.clone();
            item.music_stream_ext = "wav".into();
            item.music_stream_format_id = "fixture".into();
            item.music_stream_acodec = "pcm_s16le".into();
            item.music_cache_key = name.clone();
            state.review_capture_stream_preparation = true;
            let before = counts(&config);
            if route == "playback" {
                state.start_music_stream_playback_with_session(id, 1);
            } else {
                state.start_resolved_music_prefetch(state.queue_item_by_id(id).unwrap().clone());
            }
            let mut prepared = state
                .review_prepared_stream
                .take()
                .expect("Actual entry did not prepare stream commands");
            let mut primary = prepared.cache_command.take().unwrap();
            let mut retry = prepared.cache_cookie_free_retry_command.take().unwrap();
            let primary_args: Vec<_> = primary
                .get_args()
                .map(|v| v.to_string_lossy().into_owned())
                .collect();
            let retry_args: Vec<_> = retry
                .get_args()
                .map(|v| v.to_string_lossy().into_owned())
                .collect();
            assert_eq!(delta(counts(&config), before), [1, 1, 1]);
            assert_eq!(primary_args.iter().any(|v| v == "--cookies"), !explicit);
            assert!(!retry_args.iter().any(|v| v == "--cookies"));
            let media_before = server.media_get_count();
            // The hook stops only the audio-device handoff. Execute both real
            // prepared commands under a controlled cache owner, with no account.
            let _owner = storage::use_cache().unwrap();
            for (label, command) in [("primary", &mut primary), ("retry", &mut retry)] {
                let result = command.output().unwrap();
                fs::write(folder.join(format!("{label}.stdout.log")), &result.stdout).unwrap();
                fs::write(folder.join(format!("{label}.stderr.log")), &result.stderr).unwrap();
                assert!(
                    result.status.success(),
                    "{name}: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
            }
            assert_eq!(delta(counts(&config), before), [1, 1, 1]);
            assert_eq!(server.media_get_count() - media_before, 2);
            write_json(
                &folder.join("case-result.json"),
                serde_json::json!({"route":route,"explicit":explicit,
                "application_counts":delta(counts(&config),before),"primary_args":primary_args,"retry_args":retry_args,
                "media_gets_before":media_before,"media_gets_after":server.media_get_count(),
                "actual_entry_preparation":true,"audio_device_handoff_executed":false}),
            );
        }
    }
    let lyrics = root.join("followup-lyrics");
    fs::create_dir_all(&lyrics).unwrap();
    let config = Path::new(&tools.yt_dlp)
        .parent()
        .unwrap()
        .join("yt-dlp.conf");
    let before = counts(&config);
    let command = tools
        .prepare_music_lyrics_cache_command(
            &format!("{}/audio.wav", server.address),
            &lyrics,
            "en",
            false,
        )
        .unwrap();
    assert_eq!(delta(counts(&config), before), [1, 1, 1]);
    write_json(
        &lyrics.join("case-result.json"),
        serde_json::json!({"application_counts":delta(counts(&config),before),
        "arguments":command.get_args().map(|v|v.to_string_lossy().into_owned()).collect::<Vec<_>>(),"command_executed":false}),
    );
}
