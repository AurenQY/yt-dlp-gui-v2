//! R-OUT-012/013: one parsed configuration and the real native response envelope.
use super::review_round1::{metadata, new_state};
use super::review_round3::{choice, music_item, music_result};
use super::*;

fn receipt(folder: &Path, value: serde_json::Value) {
    fs::create_dir_all(folder).unwrap();
    fs::write(
        folder.join("case-result.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
}

pub(super) fn run(root: &Path, tools: &ToolPaths, audio: &Path, server: &Server) {
    for (ext, codec) in [("m4a", "aac"), ("webm", "libopus")] {
        assert!(
            Command::new(&tools.ffmpeg)
                .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
                .arg(audio)
                .args(["-c:a", codec])
                .arg(root.join(format!("dual-audio.{ext}")))
                .status()
                .unwrap()
                .success()
        );
    }
    for route in ["direct", "music"] {
        for case in ["comment", "config-format", "quoted", "escaped", "forbidden"] {
            let name = format!("simplicity-{route}-{case}");
            let folder = root.join(&name);
            fs::create_dir_all(&folder).unwrap();
            let url = format!("{}/dual-source.wav?{name}", server.address);
            let mut info = metadata(tools, root, &url, "Config source", true);
            for key in [
                "ext",
                "url",
                "format_id",
                "acodec",
                "vcodec",
                "protocol",
                "chapters",
            ] {
                info.as_object_mut().unwrap().remove(key);
            }
            info["formats"] = serde_json::json!([
                {"format_id":"external-aac","url":format!("{}/dual-audio.m4a",server.address),"ext":"m4a","acodec":"aac","vcodec":"none","protocol":"http","abr":96,"quality":0},
                {"format_id":"app-opus","url":format!("{}/dual-audio.webm",server.address),"ext":"webm","acodec":"opus","vcodec":"none","protocol":"http","abr":192,"quality":10}]);
            let info_path = folder.join("input.json");
            fs::write(&info_path, serde_json::to_vec(&info).unwrap()).unwrap();
            let config_path = folder.join("external.conf");
            let template = if case == "quoted" {
                "Quoted --format 100%% %(title)s.%(ext)s"
            } else {
                "%(title)s.%(ext)s"
            };
            let output = if case == "escaped" {
                r"Escaped\ space\ 100%%\ %(title)s.%(ext)s".to_owned()
            } else {
                format!("\"{template}\"")
            };
            let config_format = case == "config-format";
            fs::write(
                &config_path,
                format!(
                    "--load-info-json \"{}\"\n--output {output} # --format is only a comment\n{}{}",
                    info_path.display(),
                    if config_format {
                        "--format external-aac\n"
                    } else {
                        ""
                    },
                    if case == "forbidden" {
                        "--exec echo-forbidden\n"
                    } else {
                        ""
                    }
                ),
            )
            .unwrap();
            let mut configured = tools.clone();
            configured.yt_dlp_config = config_path.display().to_string();
            let before = crate::infrastructure::output_config_read_count(&config_path);
            let media_before = server.media_get_count();
            let (disposition, result) = if route == "direct" {
                let mut req = request(&url, &folder, "Config source");
                req.target_kind = DownloadTargetKind::Normal;
                req.video_ext = if config_format { "m4a" } else { "webm" }.into();
                req.format_selector = "app-opus".into();
                req.audio_selector = "app-opus".into();
                req.audio_ext = "webm".into();
                req.output.context.resolved = false;
                let result = download(&configured, req);
                (result.disposition, result.result)
            } else {
                let mut state = new_state(root, &configured, &name);
                let id = music_item(&mut state, &url, "Config source");
                let mut selected = choice(MusicDownloadFormat::Mp3);
                selected.mode = MusicDownloadMode::Original;
                state.start_music_download_task_at(id, selected).unwrap();
                let (_, disposition, result, _, _) = music_result(&mut state);
                (disposition, result)
            };
            let reads = crate::infrastructure::output_config_read_count(&config_path) - before;
            assert_eq!(reads, 1, "{name}: repeated config parsing");
            if case == "forbidden" {
                assert_eq!(disposition, OutputDisposition::Failed);
                assert!(result.as_ref().unwrap_err().contains("--exec"));
                assert_eq!(media_before, server.media_get_count());
            } else {
                assert_eq!(
                    disposition,
                    OutputDisposition::Published,
                    "{name}: {result:?}"
                );
                let path = Path::new(result.as_ref().unwrap());
                // Direct audio retains its established output container; probe the
                // actual selected codec as well as the music final extension.
                if route == "music" {
                    assert_eq!(
                        path.extension().unwrap(),
                        if config_format { "m4a" } else { "webm" }
                    );
                }
                if case == "quoted" {
                    assert!(
                        path.file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with("Quoted --format 100%")
                    );
                }
                let probe = Path::new(&tools.ffmpeg)
                    .parent()
                    .unwrap()
                    .join("ffprobe.exe");
                let output = Command::new(probe)
                    .args([
                        "-v",
                        "error",
                        "-show_entries",
                        "stream=codec_name",
                        "-of",
                        "json",
                    ])
                    .arg(path)
                    .output()
                    .unwrap();
                assert!(output.status.success());
                let codec =
                    serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["streams"]
                        [0]["codec_name"]
                        .as_str()
                        .unwrap()
                        .to_owned();
                assert_eq!(codec, if config_format { "aac" } else { "opus" });
                fs::write(folder.join("probe.json"), output.stdout).unwrap();
            }
            receipt(
                &folder,
                serde_json::json!({"route":route,"case":case,"config_owned_format":config_format,
                "application_config_reads":reads,"disposition":format!("{disposition:?}"),"result":result,
                "media_gets_before":media_before,"media_gets_after":server.media_get_count()}),
            );
        }
    }
    native_envelopes(root, tools, server);
    println!("Simplicity output acceptance passed: {}", root.display());
}

fn native_envelopes(root: &Path, tools: &ToolPaths, server: &Server) {
    for case in [
        "validation",
        "publication",
        "before-publication",
        "success",
        "skip",
        "cancel",
    ] {
        let folder = root.join(format!("simplicity-native-{case}"));
        let external = folder.join("external");
        let destination = folder.join("final");
        fs::create_dir_all(&external).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(external.join("keep.txt"), b"external preserve").unwrap();
        let url = "http://output-fixture.invalid/a.mp4";
        let mut info = metadata(tools, root, url, "Envelope", false);
        info["subtitles"] = serde_json::json!({"en":[{"ext":"vtt","url":format!("{}/captions.vtt",server.address)}]});
        let input = folder.join("input.json");
        fs::write(&input, serde_json::to_vec(&info).unwrap()).unwrap();
        let path = folder.join("native.conf");
        fs::write(
            &path,
            format!(
                "--load-info-json \"{}\"\n{}",
                input.display(),
                if case == "before-publication" {
                    "--exec forbidden\n"
                } else {
                    ""
                }
            ),
        )
        .unwrap();
        let mut config = AppConfig::default()
            .with_fixture_file(crate::infrastructure::runtime_config_file_path());
        config.download_dir = destination.display().to_string();
        config.yt_dlp_path = tools.yt_dlp.clone();
        config.ffmpeg_path = tools.ffmpeg.clone();
        config.yt_dlp_config_path = path.display().to_string();
        config.use_browser_cookies = false;
        config.thumbnail_mode = crate::infrastructure::PostProcessMode::Off;
        config.proxy_enabled = true;
        config.proxy_url = server.address.clone();
        config.output_existing_file = if case == "skip" {
            ExistingFilePolicy::Skip
        } else {
            ExistingFilePolicy::KeepBoth
        };
        config.save().unwrap();
        if case == "skip" {
            fs::write(destination.join("Envelope.mp4"), b"established final").unwrap();
        }
        let mut request = serde_json::json!({"url":url,"videoFormatId":"fixture","videoIsMuxed":true,
            "videoExt":"mp4","audioExt":"m4a","outputFormat":"mp4","fileName":"Envelope.mp4",
            "thumbnailMode":"off","subtitleLanguage":"en","subtitleMode":"download","chapterMode":"off","useAria2":false});
        if matches!(case, "validation" | "publication") {
            request["reviewPublicationFailure"] = case.into();
            request["reviewExternal"] = external.display().to_string().into();
            request["reviewErrorMessage"] =
                "Arbitrary wording without a published-files marker".into();
        }
        let cancelled = Arc::new(AtomicBool::new(case == "cancel"));
        let before_gets = server.media_get_count();
        let source_hash = crate::infrastructure::sha256_hex_file(&root.join("source.mp4")).unwrap();
        let frames = crate::infrastructure::browser_native_messaging::output_review_native_envelope(
            &request, &cancelled,
        );
        fs::write(
            folder.join("response-frames.json"),
            serde_json::to_vec_pretty(&frames).unwrap(),
        )
        .unwrap();
        let response = frames.last().unwrap();
        let expected = match case {
            "validation" | "publication" => "PartialFailure",
            "before-publication" => "Failed",
            "skip" => "SkippedExisting",
            "cancel" => "Cancelled",
            _ => "Published",
        };
        assert_eq!(response["data"]["outcome"], expected, "{case}: {response}");
        if matches!(case, "validation" | "publication") {
            assert_eq!(response["ok"], false);
            let paths = response["data"]["outputPaths"].as_array().unwrap();
            assert_eq!(paths.len(), 1);
            assert!(Path::new(paths[0].as_str().unwrap()).is_file());
            let initial: serde_json::Value =
                serde_json::from_slice(&fs::read(folder.join("first-published.json")).unwrap())
                    .unwrap();
            assert_eq!(initial["path"], paths[0]);
            assert_eq!(
                crate::infrastructure::sha256_hex_file(Path::new(paths[0].as_str().unwrap()))
                    .unwrap(),
                initial["sha256"]
            );
            if case == "publication" {
                let later = if Path::new(paths[0].as_str().unwrap()).extension().unwrap() == "mp4" {
                    destination.join("Envelope.en.srt")
                } else {
                    destination.join("Envelope.mp4")
                };
                assert_eq!(fs::read(later).unwrap(), b"late external sentinel");
            }

            assert!(
                !frames
                    .iter()
                    .any(|frame| frame["data"]["event"] == "complete")
            );
            assert!(
                response["message"]
                    .as_str()
                    .unwrap()
                    .starts_with("Arbitrary wording")
            );
        }
        if case == "skip" {
            assert_eq!(
                fs::read(destination.join("Envelope.mp4")).unwrap(),
                b"established final"
            );
        }
        assert_eq!(
            fs::read(external.join("keep.txt")).unwrap(),
            b"external preserve"
        );
        fs::write(
            folder.join("response-frames.json"),
            serde_json::to_vec_pretty(&frames).unwrap(),
        )
        .unwrap();
        assert_eq!(
            crate::infrastructure::sha256_hex_file(&root.join("source.mp4")).unwrap(),
            source_hash
        );
        if matches!(case, "skip" | "cancel" | "before-publication") {
            assert_eq!(server.media_get_count(), before_gets);
        }
        receipt(
            &folder,
            serde_json::json!({"case":case,"expected":expected,"response":response,"source_sha256":source_hash,"media_gets_before":before_gets,"media_gets_after":server.media_get_count()}),
        );
    }
}
