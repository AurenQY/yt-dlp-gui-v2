//! R-OUT-012: whole metadata operations with the real yt-dlp generic extractor.
use super::*;
use serde_json::{Value, json};

fn save(path: &Path, value: Value) {
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

fn counts(path: &Path) -> [usize; 3] {
    crate::infrastructure::output_config_observation_counts(path)
}

fn delta(after: [usize; 3], before: [usize; 3]) -> [usize; 3] {
    std::array::from_fn(|i| after[i] - before[i])
}

fn native_config(root: &Path, tools: &ToolPaths, server: &Server) {
    fs::create_dir_all(root.join("final")).unwrap();
    let mut config =
        AppConfig::default().with_fixture_file(crate::infrastructure::runtime_config_file_path());
    config.yt_dlp_path = tools.yt_dlp.clone();
    config.yt_dlp_config_path = tools.yt_dlp_config.clone();
    config.ffmpeg_path = tools.ffmpeg.clone();
    config.download_dir = root.join("final").display().to_string();
    config.cache_location_mode =
        serde_json::from_value(json!(if root.file_name().unwrap() == "temporary" {
            "windows_temp"
        } else {
            "v2_cache"
        }))
        .unwrap();
    config.cache_dir = root.join("app/cache").display().to_string();
    config.proxy_enabled = true;
    config.proxy_url = server.address.clone();
    config.use_browser_cookies = false;
    config.save().unwrap();
}

fn requests(server: &Server) -> Vec<Value> {
    fs::read_to_string(server.root.join("http-requests.jsonl"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn playlist(
    root: &Path,
    folder: &Path,
    tools: &ToolPaths,
    config: &Path,
    server: &Server,
    streaming: bool,
    fresh: bool,
) {
    native_config(root, tools, server);
    let request = json!({"action":"analyze","backend":"y","playlistScope":"playlist",
        "itemLimit":2,"url":"http://metadata-fixture.invalid/metadata/playlist/feed.xml"});
    let before = counts(config);
    let http_before = requests(server).len();
    let value = crate::infrastructure::browser_native_messaging::output_review_native_analysis(
        &request, streaming,
    )
    .unwrap();
    let observed = delta(counts(config), before);
    save(
        &folder.join(format!("playlist-{streaming}-{fresh}.json")),
        json!({
            "request":request,"response":value,"application_counts":observed,
            "http_requests":&requests(server)[http_before..],"fresh_config":fresh
        }),
    );
    assert_eq!(observed, [1, 1, 1]);
    let items: Vec<Value> = if streaming {
        assert_eq!(value["summary"]["itemCount"], 2);
        value["frames"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|frame| frame["data"]["event"] == "item")
            .map(|frame| frame["data"]["item"].clone())
            .collect()
    } else {
        value["items"].as_array().unwrap().clone()
    };
    assert_eq!(items.len(), 2);
    for (item, title) in items.iter().zip(["Metadata A", "Metadata B"]) {
        let actual = item["title"].as_str().unwrap();
        assert!(actual.contains(title), "{actual}");
        assert!(!item["formats"].as_array().unwrap().is_empty());
    }
    let http = requests(server);
    assert!(http[http_before..].iter().all(|r| r["user_agent"]
        == if fresh {
            "Metadata-Fresh"
        } else {
            "Metadata-Initial"
        }));
    let get_paths: Vec<_> = http[http_before..]
        .iter()
        .filter(|r| r["method"] == "GET")
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        get_paths,
        [
            "/metadata/playlist/feed.xml",
            "/metadata/playlist/a.html",
            "/metadata/playlist/b.html"
        ]
    );
}

fn music(
    folder: &Path,
    tools: &ToolPaths,
    config: &Path,
    server: &Server,
    retry: bool,
    direct: bool,
    fresh: bool,
) {
    let name = if retry { "retry" } else { "single" };
    let source = format!(
        "http://metadata-fixture.invalid/metadata/{}/{name}.html",
        folder.file_name().unwrap().to_str().unwrap()
    );
    assert_eq!(
        crate::infrastructure::extractor_index::matching_yt_source_scope(&source).unwrap(),
        crate::infrastructure::extractor_index::YtSourceScope::Unknown
    );
    let before = counts(config);
    let http_before = requests(server).len();
    let (title, direct_url, attempts) = if direct {
        let result = tools
            .analyze_music_stream_url_detailed(&source, false)
            .unwrap_or_else(|error| panic!("{}", error.message));
        (
            result.json["title"].as_str().unwrap().to_owned(),
            result.json["url"].as_str().unwrap().to_owned(),
            json!([{"command":result.command_line,"success":true}]),
        )
    } else {
        let result = resolve_music_stream_seed_with_cookie_fallback(tools, &source, retry);
        let attempts = result
            .attempts
            .iter()
            .map(|a| json!({"command":a.command_line,"success":a.success}))
            .collect::<Vec<_>>();
        save(
            &folder.join(format!("music-attempts-{retry}-{fresh}.json")),
            json!(attempts),
        );
        let seed = result.result.unwrap();
        assert_eq!(seed.cookies_fallback, retry);
        assert!(!seed.used_cookies);
        assert_eq!(attempts.len(), if retry { 2 } else { 1 });
        if retry {
            assert_eq!(attempts[0]["success"], false);
            assert_eq!(
                attempts[0]["command"]
                    .as_str()
                    .unwrap()
                    .contains("--cookies"),
                !tools.has_explicit_config()
            );
        }
        assert_eq!(attempts.last().unwrap()["success"], true);
        assert!(
            !attempts.last().unwrap()["command"]
                .as_str()
                .unwrap()
                .contains("--cookies")
        );
        (seed.naming_metadata.title, seed.direct_url, json!(attempts))
    };
    let observed = delta(counts(config), before);
    let http = requests(server);
    save(
        &folder.join(format!("music-{retry}-{direct}-{fresh}.json")),
        json!({
            "source":source,"title":title,"direct_url":direct_url,"attempts":attempts,
            "application_counts":observed,"http_requests":&http[http_before..],"fresh_config":fresh
        }),
    );
    assert_eq!(observed, [1, 1, 1]);
    assert!(title.contains(if retry {
        "Metadata RETRY"
    } else {
        "Metadata SINGLE"
    }));
    assert!(http[http_before..].iter().all(|r| r["user_agent"]
        == if fresh {
            "Metadata-Fresh"
        } else {
            "Metadata-Initial"
        }));
    assert!(direct_url.ends_with("/source.flac"));
    let gets: Vec<_> = http[http_before..]
        .iter()
        .filter(|r| r["method"] == "GET")
        .collect();
    assert_eq!(gets.len(), if retry { 4 } else { 2 });
    if retry {
        assert_eq!(
            gets.iter()
                .map(|r| r["status"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            [200, 403, 200, 200]
        );
        assert_eq!(
            gets.iter()
                .map(|r| r["cookie_present"].as_bool().unwrap())
                .collect::<Vec<_>>(),
            [
                !tools.has_explicit_config(),
                !tools.has_explicit_config(),
                false,
                false
            ]
        );
    }
}

pub(super) fn run(root: &Path, tools: &ToolPaths, server: &Server) {
    let generated = Command::new(&tools.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(root.join("source.wav"))
        .arg(root.join("source.flac"))
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let resources = root.join("app/resources");
    fs::create_dir_all(&resources).unwrap();
    save(
        &resources.join("extractor-index.json"),
        json!({"y":[["MetadataUnknown",r"http://metadata-fixture\.invalid/metadata/"]],"ys":[],"yd":0,"g":[["Never",r"(?!)"]]}),
    );
    let default_output = root.join("app/existing.wav");
    fs::write(&default_output, b"established metadata sentinel").unwrap();
    let media_before = server.media_get_count();
    for explicit in [false, true] {
        let folder = root.join(format!(
            "metadata-{}",
            if explicit { "explicit" } else { "portable" }
        ));
        fs::create_dir_all(&folder).unwrap();
        let config = if explicit {
            folder.join("external.conf")
        } else {
            Path::new(&tools.yt_dlp)
                .parent()
                .unwrap()
                .join("yt-dlp.conf")
        };
        let content = "--user-agent Metadata-Initial\n--no-simulate\n--force-overwrites\n-o existing.wav\n--retries 0\n--extractor-retries 0\n";
        fs::write(&config, content).unwrap();
        fs::write(folder.join("config-before.conf"), content).unwrap();
        let cookie = folder.join("cookies.txt");
        fs::write(&cookie, "# Netscape HTTP Cookie File\nmetadata-fixture.invalid\tFALSE\t/\tFALSE\t2147483647\tfixture\t1\n").unwrap();
        let mut selected = tools.clone();
        selected.yt_dlp_config = if explicit {
            config.display().to_string()
        } else {
            String::new()
        };
        selected.proxy_enabled = true;
        selected.proxy_url = server.address.clone();
        selected.browser_cookie_source = "file".into();
        selected.browser_cookie_file = cookie.display().to_string();
        playlist(root, &folder, &selected, &config, server, false, false);
        playlist(root, &folder, &selected, &config, server, true, false);
        music(&folder, &selected, &config, server, false, false, false);
        music(&folder, &selected, &config, server, true, false, false);
        music(&folder, &selected, &config, server, false, true, false);
        assert_eq!(fs::read(&config).unwrap(), content.as_bytes());
        // A new operation observes a changed portable file or a newly selected explicit file.
        let fresh = if explicit {
            folder.join("fresh.conf")
        } else {
            config.clone()
        };
        let fresh_content = content.replace("Metadata-Initial", "Metadata-Fresh");
        fs::write(&fresh, &fresh_content).unwrap();
        fs::write(folder.join("config-fresh.conf"), &fresh_content).unwrap();
        if explicit {
            selected.yt_dlp_config = fresh.display().to_string();
        }
        playlist(root, &folder, &selected, &fresh, server, false, true);
        music(&folder, &selected, &fresh, server, false, false, true);
        assert_eq!(fs::read(&fresh).unwrap(), fresh_content.as_bytes());
        let rejected = format!("{content}--exec \"echo forbidden\"\n");
        fs::write(&fresh, &rejected).unwrap();
        fs::write(folder.join("config-rejected.conf"), &rejected).unwrap();
        native_config(root, &selected, server);
        let http_before = requests(server).len();
        let mut failures = Vec::new();
        for route in ["native", "native-stream", "music", "music-direct"] {
            let before = counts(&fresh);
            let error = if route.starts_with("native") {
                crate::infrastructure::browser_native_messaging::output_review_native_analysis(
                    &json!({"action":"analyze","playlistScope":"playlist","itemLimit":2,
                        "url":"http://metadata-fixture.invalid/metadata/playlist/feed.xml"}),
                    route == "native-stream",
                )
                .unwrap_err()
            } else if route == "music-direct" {
                selected
                    .analyze_music_stream_url_detailed(
                        "http://metadata-fixture.invalid/metadata/single.html",
                        true,
                    )
                    .err()
                    .unwrap()
                    .message
            } else {
                let result = resolve_music_stream_seed_with_cookie_fallback(
                    &selected,
                    "http://metadata-fixture.invalid/metadata/retry.html",
                    true,
                );
                assert!(result.attempts.is_empty());
                result.result.err().unwrap()
            };
            assert!(error.contains("--exec"), "{route}: {error}");
            assert_eq!(delta(counts(&fresh), before), [1, 1, 1]);
            failures.push(json!({"route":route,"error":error,"application_counts":delta(counts(&fresh),before)}));
        }
        assert_eq!(requests(server).len(), http_before);
        assert_eq!(fs::read(&fresh).unwrap(), rejected.as_bytes());
        assert_eq!(
            fs::read(&default_output).unwrap(),
            b"established metadata sentinel"
        );
        assert_eq!(server.media_get_count(), media_before);
        assert!(!folder.join("existing.wav").exists());
        save(
            &folder.join("case-result.json"),
            json!({"explicit":explicit,"successful_operations":7,
            "rejected":failures,"media_gets_before":media_before,"media_gets_after":server.media_get_count(),
            "sentinel_unchanged":true,"config_unchanged_during_operations":true,"no_http_on_rejection":true}),
        );
    }
    println!(
        "R-OUT-012 complete metadata-operation acceptance passed: {}",
        root.display()
    );
}
