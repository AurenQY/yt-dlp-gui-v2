//! Focused regression evidence for R-CACHE-002 through R-CACHE-004.
use super::*;
use crate::infrastructure::{
    AppConfig, browser_native_messaging,
    gallery_dl::{GalleryDlBackend, GalleryDlRuntime},
    tools::{DownloadRequest, DownloadTargetKind, ToolPaths},
};
use std::process::Command;

#[test]
fn review_output_and_link_boundaries() {
    let (app, temp) = fixture("review-paths");
    for mode in ["portable", "temporary", "restart"] {
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "infrastructure::storage_paths::tests::review_repairs::review_paths_child",
                "--nocapture",
            ])
            .env("YT_DLP_GUI_REVIEW_APP", &app)
            .env("YT_DLP_GUI_REVIEW_TEMP", &temp)
            .env("YT_DLP_GUI_REVIEW_MODE", mode)
            .output()
            .unwrap();
        println!("{}", String::from_utf8_lossy(&result.stdout));
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    println!(
        "R-CACHE-002/003/004 fixture: {}",
        app.parent().unwrap().display()
    );
}

#[test]
#[ignore = "Set YT_DLP_GUI_TEST_PYTHON and YT_DLP_GUI_GALLERY_TEST_PACKAGES; actual direct yt-dlp information cache"]
fn review_real_information_cache() {
    let (app, temp) = fixture("review-real-info");
    for mode in ["portable", "temporary"] {
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "infrastructure::storage_paths::tests::review_repairs::review_paths_child",
                "--nocapture",
            ])
            .env("YT_DLP_GUI_REVIEW_APP", &app)
            .env("YT_DLP_GUI_REVIEW_TEMP", &temp)
            .env("YT_DLP_GUI_REVIEW_MODE", mode)
            .env("YT_DLP_GUI_REVIEW_REAL", "1")
            .output()
            .unwrap();
        println!("{}", String::from_utf8_lossy(&result.stdout));
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    println!(
        "Real direct information-cache fixture: {}",
        app.parent().unwrap().display()
    );
}

#[test]
fn review_paths_child() {
    let Some(app) = std::env::var_os("YT_DLP_GUI_REVIEW_APP") else {
        return;
    };
    let app = PathBuf::from(app);
    let temp = PathBuf::from(std::env::var_os("YT_DLP_GUI_REVIEW_TEMP").unwrap());
    let phase = std::env::var("YT_DLP_GUI_REVIEW_MODE").unwrap();
    let mode = if phase == "portable" {
        CacheLocationMode::V2Cache
    } else {
        CacheLocationMode::WindowsTemp
    };
    initialize_fixture(&app, &temp, mode, "cache");
    let paths = require_ready().unwrap();
    let final_root = app.join(format!("user exports 空间/{phase}"));
    if phase == "restart" {
        for mode in ["portable", "temporary"] {
            let root = app.join(format!("user exports 空间/{mode}"));
            assert_eq!(
                fs::read(root.join("final.mp4")).unwrap(),
                b"completed user media"
            );
            assert_eq!(
                fs::read(root.join("notes.txt")).unwrap(),
                b"unknown user file at final-output boundary"
            );
        }
        println!("R-CACHE-004: user outputs and unknown neighboring files survive restart");
        return;
    }
    let fake_tool = app.join("fixture-tools/yt-dlp.exe");
    write(&fake_tool, b"preparation only; must not execute");
    let mut tools = ToolPaths {
        yt_dlp: fake_tool.display().to_string(),
        ..Default::default()
    };
    let request = request(&final_root);
    for child in ["yt-dlp", "yt-dlp-temp"] {
        let managed = paths.cache_root.join(child);
        if managed.exists() {
            remove_owned_tree(&managed).unwrap();
        }
        let outside = app
            .parent()
            .unwrap()
            .join(format!("outside-{phase}-{child}"));
        write(&outside.join("sentinel.bin"), b"outside sentinel");
        #[cfg(windows)]
        {
            let result = Command::new("cmd.exe")
                .args(["/d", "/c", "mklink", "/J"])
                .arg(managed.display().to_string().replace('/', "\\"))
                .arg(outside.display().to_string().replace('/', "\\"))
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        #[cfg(not(windows))]
        std::os::unix::fs::symlink(&outside, &managed).unwrap();
        let analysis = tools
            .prepare_batch_add_command("reviewcache:fixture", false)
            .err()
            .unwrap();
        let download = tools.prepare_download(&request).err().unwrap();
        assert!(analysis.contains("Storage links"), "{analysis}");
        assert!(download.contains("Storage links"), "{download}");
        assert_eq!(
            fs::read(outside.join("sentinel.bin")).unwrap(),
            b"outside sentinel"
        );
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
        #[cfg(windows)]
        fs::remove_dir(&managed).unwrap();
        #[cfg(not(windows))]
        fs::remove_file(&managed).unwrap();
    }
    println!(
        "R-CACHE-003: both linked children reject production analysis/download preparation; outside sentinels intact ({phase})"
    );
    let config_file = app.join("output-review.conf");
    tools.yt_dlp_config = config_file.display().to_string();
    for template in [
        "~/preview/%(id)s.%(ext)s",
        "thumbnail:~\\preview\\%(id)s.jpg",
        "subtitle:~SKY/preview/%(id)s.srt",
    ] {
        write(&config_file, format!("-o '{template}'\n").as_bytes());
        let error = tools.prepare_download(&request).err().unwrap();
        assert!(error.contains("environment-dependent"), "{error}");
    }
    write(
        &config_file,
        b"-o 'relative/%(id)s.%(ext)s'\n--no-cache-dir\n",
    );
    let mut supported = request.clone();
    supported.output.context.id = "fixture-id".into();
    let safe = tools.prepare_download(&supported).unwrap();
    assert!(
        safe.command
            .get_args()
            .any(|value| value == "--no-cache-dir")
    );
    drop(safe);
    let cache_export = paths.cache_root.join("audio/user-export");
    write(
        &config_file,
        format!("-P 'home:{}'\n", cache_export.display()).as_bytes(),
    );
    assert!(
        tools
            .prepare_download(&request)
            .err()
            .unwrap()
            .contains("outside application cache")
    );
    tools.yt_dlp_config.clear();
    let config_path = crate::infrastructure::config::runtime_config_file_path();
    let mut config = AppConfig::default().with_fixture_file(config_path.clone());
    config.download_dir = final_root.display().to_string();
    config.yt_dlp_path = fake_tool.display().to_string();
    config.save().unwrap();
    let mut state = crate::app::state::AppState::from_runtime(config.clone(), tools.clone());
    let previous = state.config.download_dir.clone();
    state.set_output_dir(cache_export.display().to_string());
    assert_eq!(state.config.download_dir, previous);
    assert!(state.last_action.contains("outside application cache"));
    assert!(!cache_export.exists());
    let gallery = GalleryDlBackend::new(GalleryDlRuntime::standalone(
        app.join("missing-gallery.exe"),
    ));
    for root in [
        &paths.cache_root,
        &paths.portable_root,
        &paths.temporary_root,
    ] {
        let bad = root.join("audio/user-export");
        let mut direct = request.clone();
        direct.output_dir = bad.display().to_string();
        assert!(
            tools
                .prepare_download(&direct)
                .err()
                .unwrap()
                .contains("outside application cache")
        );
        direct.output_dir = final_root.display().to_string();
        direct.output_path = Some(bad.join("final.mp4").display().to_string());
        assert!(
            tools
                .prepare_download(&direct)
                .err()
                .unwrap()
                .contains("outside application cache")
        );
        assert!(
            gallery
                .prepare_item_download("storagefixture:http", &bad, &tools, false)
                .err()
                .unwrap()
                .contains("outside application cache")
        );
        let mut native = config.clone();
        native.download_dir = bad.display().to_string();
        assert!(
            browser_native_messaging::review_configured_output(&native)
                .unwrap_err()
                .contains("outside application cache")
        );
        assert!(!bad.exists());
    }
    // Test completed-output ownership at its actual boundary, with an unknown
    // neighboring user file, after a mutable output selection in this process.
    state.set_output_dir(final_root.display().to_string());
    tools.prepare_download(&request).unwrap();
    write(&final_root.join("final.mp4"), b"completed user media");
    write(
        &final_root.join("notes.txt"),
        b"unknown user file at final-output boundary",
    );
    write(
        &paths.cache_root.join("audio/disposable/audio.mp4"),
        b"disposable cache",
    );
    state.clear_music_stream_cache();
    state.clear_app_cache();
    assert_eq!(
        fs::read(final_root.join("final.mp4")).unwrap(),
        b"completed user media"
    );
    assert_eq!(
        fs::read(final_root.join("notes.txt")).unwrap(),
        b"unknown user file at final-output boundary"
    );
    assert!(!paths.cache_root.join("audio/disposable/audio.mp4").exists());
    assert!(
        fs::read_to_string(config_path)
            .unwrap()
            .contains("user exports")
    );
    if std::env::var_os("YT_DLP_GUI_REVIEW_REAL").is_some() {
        real_information_cache(&app, &temp, &tools);
    }
    println!(
        "R-CACHE-002/004: home expansion rejected; output changes, item/native/gallery/cache-root aliases rejected before output writes; safe outputs and unknown neighbors survive audio/all cleanup ({phase})"
    );
}

fn real_information_cache(app: &Path, temp: &Path, tools: &ToolPaths) {
    let script = app.join("direct-info-fixture.py");
    write(&script, br#"import sys
import yt_dlp
from yt_dlp.extractor.common import InfoExtractor
from yt_dlp.globals import extractors
class ReviewCacheIE(InfoExtractor):
    _VALID_URL = r'reviewcache:(?P<id>fixture)'
    def _real_extract(self, url):
        self._downloader.cache.store('review-cache', 'generated', {'synthetic': True})
        return {'id':'fixture', 'title':'fixture', 'url':'http://127.0.0.1/never-transferred.mp4', 'ext':'mp4'}
yt_dlp.extractor.import_extractors()
extractors.value = {'ReviewCacheIE':ReviewCacheIE, **extractors.value}
print('yt-dlp ' + yt_dlp.version.__version__, file=sys.stderr)
yt_dlp.main()
"#);
    let prepared = tools
        .prepare_batch_add_command("reviewcache:fixture", false)
        .unwrap();
    let output = Command::new(std::env::var_os("YT_DLP_GUI_TEST_PYTHON").unwrap())
        .arg("-B")
        .arg(script)
        .args(prepared.get_args())
        .env(
            "PYTHONPATH",
            std::env::var_os("YT_DLP_GUI_GALLERY_TEST_PACKAGES").unwrap(),
        )
        .env("TEMP", temp)
        .env("TMP", temp)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let generated = paths()
        .cache_root
        .join("yt-dlp/review-cache/generated.json");
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&generated).unwrap()).unwrap();
    assert_eq!(value["data"]["synthetic"], true);
    assert!(!app.join("never-transferred.mp4").exists());
    println!(
        "{}: generated information cache at {}",
        String::from_utf8_lossy(&output.stderr).trim(),
        generated.display()
    );
}

fn request(output: &Path) -> DownloadRequest {
    DownloadRequest {
        output: Default::default(),
        target_kind: DownloadTargetKind::Video,
        url: "http://127.0.0.1/fixture.mp4".into(),
        playlist_index: None,
        format_selector: "best".into(),
        video_selector: "best".into(),
        audio_selector: String::new(),
        is_muxed_video: true,
        video_ext: "mp4".into(),
        audio_ext: String::new(),
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
        output_dir: output.display().to_string(),
        file_name: "final.mp4".into(),
        download_section_args: Vec::new(),
    }
}
