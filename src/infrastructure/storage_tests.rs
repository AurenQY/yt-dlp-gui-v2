use super::*;
use crate::infrastructure::{component_update, storage_migration, yaml_store};
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "storage_tool_tests.rs"]
mod real_tools;
#[path = "storage_review_tests.rs"]
mod review_repairs;
#[path = "storage_supplement_tests.rs"]
mod supplement;

fn fixture(name: &str) -> (PathBuf, PathBuf) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/cache-location-validation")
        .join(format!(
            "{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    let app = root.join("app");
    let temp = root.join("system-temp");
    fs::create_dir_all(&app).unwrap();
    fs::create_dir_all(&temp).unwrap();
    (app, temp)
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn storage_location_identity_and_missing_portable_directory() {
    let (app, temp) = fixture("roots");
    let portable = resolve(&app, &temp, CacheLocationMode::V2Cache, "missing cache").unwrap();
    assert_eq!(portable.cache_root, app.join("missing cache"));
    assert!(!portable.cache_root.exists());
    let temporary = resolve(&app, &temp, CacheLocationMode::WindowsTemp, "missing cache").unwrap();
    assert_eq!(portable.installation_id, temporary.installation_id);
    assert!(temporary.cache_root.starts_with(temp.join("yt-dlp-gui")));
    let other = app.with_file_name("other-app");
    fs::create_dir_all(&other).unwrap();
    assert_ne!(
        temporary.installation_id,
        resolve(&other, &temp, CacheLocationMode::WindowsTemp, "cache")
            .unwrap()
            .installation_id
    );
    assert_eq!(
        temporary.installation_id,
        resolve(
            &app.join("."),
            &temp,
            CacheLocationMode::WindowsTemp,
            "cache"
        )
        .unwrap()
        .installation_id
    );
    for unsafe_root in [
        &app,
        &app.join("state"),
        &app.join("tools"),
        &temp,
        app.parent().unwrap(),
    ] {
        assert!(
            resolve(
                &app,
                &temp,
                CacheLocationMode::V2Cache,
                &unsafe_root.display().to_string()
            )
            .is_err(),
            "{}",
            unsafe_root.display()
        );
    }
}

#[test]
fn storage_migration_is_non_destructive_and_repeatable() {
    let (app, temp) = fixture("migration");
    let paths = resolve(&app, &temp, CacheLocationMode::V2Cache, "custom-cache").unwrap();
    let source = app.join("cache/manifest/installed_app_version.yaml");
    write(&source, b"version: old\ninstalled_at_unix: 1\n");
    write(&app.join("cache/manifest/installed.yaml"), b"entries: []\n");
    write(
        &app.join("custom-cache/state/music-cue-memory.yaml"),
        b"version: 1\nentries: {}\n",
    );
    write(&app.join("cache/audio/keep.bin"), b"old audio");
    storage_migration::migrate(&paths).unwrap();
    assert!(storage_migration::is_complete(&paths).unwrap());
    let destination = paths
        .state_root
        .join("component-update/installed_app_version.yaml");
    let first = fs::read(&destination).unwrap();
    assert!(source.exists());
    assert!(paths.state_root.join("music-cue-memory.yaml").exists());
    write(&source, b"version: stale\ninstalled_at_unix: 0\n");
    storage_migration::migrate(&paths).unwrap();
    assert_eq!(first, fs::read(destination).unwrap());
    assert_eq!(
        fs::read(app.join("cache/audio/keep.bin")).unwrap(),
        b"old audio"
    );
}

#[test]
fn storage_migration_keeps_existing_state_and_rejects_invalid_schema() {
    let (app, temp) = fixture("invalid-state");
    let paths = resolve(&app, &temp, CacheLocationMode::V2Cache, "cache").unwrap();
    write(
        &app.join("cache/manifest/installed_app_version.yaml"),
        b"version: old\ninstalled_at_unix: 1\n",
    );
    let destination = paths
        .state_root
        .join("component-update/installed_app_version.yaml");
    write(&destination, b"version: newer\ninstalled_at_unix: 2\n");
    write(
        &app.join("cache/state/music-cue-memory.yaml"),
        b"entries: invalid\n",
    );
    assert!(storage_migration::migrate(&paths).is_err());
    assert!(!storage_migration::is_complete(&paths).unwrap());
    assert_eq!(
        fs::read(&destination).unwrap(),
        b"version: newer\ninstalled_at_unix: 2\n"
    );
    write(
        &app.join("cache/state/music-cue-memory.yaml"),
        b"version: 1\nentries: {}\n",
    );
    storage_migration::migrate(&paths).unwrap();
    assert_eq!(
        fs::read(&destination).unwrap(),
        b"version: newer\ninstalled_at_unix: 2\n"
    );
}

fn pending(paths: &RuntimeStoragePaths) -> component_update::PendingAppUpdateManifest {
    component_update::PendingAppUpdateManifest {
        state: component_update::PendingAppUpdateState::ApplyRequested,
        version: "2099.01.01".to_owned(),
        staged_exe: paths
            .portable_root
            .join("temp/manifest/staged/app/yt-dlp-gui.exe"),
        target_exe: paths.application_root.join("yt-dlp-gui.exe"),
        backup_exe: paths
            .application_root
            .join("cache/manifest/backup/yt-dlp-gui.old.exe"),
        release_url: None,
        downloaded_at_unix: 1,
        attempt_count: 1,
        last_error: None,
    }
}

#[test]
fn storage_migration_preserves_pending_stage_and_copies_recovery_before_rebinding() {
    let (app, temp) = fixture("pending");
    let paths = resolve(&app, &temp, CacheLocationMode::WindowsTemp, "cache").unwrap();
    let original = pending(&paths);
    write(&original.staged_exe, b"staged fixture");
    write(&original.backup_exe, b"recovery fixture");
    write(
        &app.join("cache/manifest/pending_app_update.yaml"),
        serde_yaml::to_string(&original).unwrap().as_bytes(),
    );
    storage_migration::migrate(&paths).unwrap();
    let migrated: component_update::PendingAppUpdateManifest = yaml_store::read_yaml_file(
        &paths
            .state_root
            .join("component-update/pending_app_update.yaml"),
    )
    .unwrap();
    assert_eq!(original.staged_exe, migrated.staged_exe);
    assert_eq!(fs::read(&migrated.backup_exe).unwrap(), b"recovery fixture");
    assert!(original.backup_exe.exists());
    component_update::validate_pending_storage(&migrated, &paths).unwrap();
    let mut invalid = migrated;
    invalid.staged_exe = app.parent().unwrap().join("outside.exe");
    assert!(component_update::validate_pending_storage(&invalid, &paths).is_err());
}

#[test]
fn storage_cleanup_keeps_unknown_state_inactive_and_pending_files() {
    let (app, temp) = fixture("cleanup");
    let paths = resolve(&app, &temp, CacheLocationMode::V2Cache, "cache").unwrap();
    for relative in [
        "audio/a.bin",
        "queue-thumbnails-v1/a.png",
        "temp/manifest/staged/app/new.exe",
    ] {
        write(&paths.cache_root.join(relative), b"cache");
    }
    for path in [
        paths.cache_root.join("user.txt"),
        paths.cache_root.join("manifest/pending_app_update.yaml"),
        paths.cache_root.join("state/music-cue-memory.yaml"),
        paths.state_root.join("keep.yaml"),
        paths.temporary_root.join("audio/inactive.bin"),
    ] {
        write(&path, b"preserve");
    }
    let (entries, bytes) = clear_disposable(
        &paths,
        &[paths.cache_root.join("temp/manifest/staged/app/new.exe")],
    )
    .unwrap();
    assert_eq!((entries, bytes), (2, 10));
    assert!(!paths.cache_root.join("audio").exists());
    assert!(paths.cache_root.join("user.txt").exists());
    assert!(
        paths
            .cache_root
            .join("manifest/pending_app_update.yaml")
            .exists()
    );
    assert!(
        paths
            .cache_root
            .join("state/music-cue-memory.yaml")
            .exists()
    );
    assert!(
        paths
            .cache_root
            .join("temp/manifest/staged/app/new.exe")
            .exists()
    );
    assert!(paths.temporary_root.join("audio/inactive.bin").exists());
    assert!(paths.state_root.join("keep.yaml").exists());
}

#[test]
fn storage_os_locks_exclude_cleanup_during_active_work() {
    let (app, _) = fixture("locks");
    let lock = app.join("state/runtime/cache-use.lock");
    let reader = open_lock(&lock).unwrap();
    reader.try_lock_shared().unwrap();
    let another = open_lock(&lock).unwrap();
    another.try_lock_shared().unwrap();
    let exclusive = open_lock(&lock).unwrap();
    assert!(exclusive.try_lock().is_err());
    drop(reader);
    drop(another);
    exclusive.try_lock().unwrap();
    assert!(open_lock(&lock).unwrap().try_lock_shared().is_err());
}

#[cfg(windows)]
#[test]
fn storage_junction_cleanup_cannot_reach_another_folder() {
    let (app, temp) = fixture("junction");
    let paths = resolve(&app, &temp, CacheLocationMode::V2Cache, "cache").unwrap();
    fs::create_dir_all(&paths.cache_root).unwrap();
    let outside = app.parent().unwrap().join("outside");
    write(&outside.join("keep.txt"), b"outside sentinel");
    let link = paths.cache_root.join("audio");
    let output = std::process::Command::new("cmd.exe")
        .args(["/C", "mklink", "/J"])
        .arg(link.display().to_string().replace('/', "\\"))
        .arg(outside.display().to_string().replace('/', "\\"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(clear_disposable(&paths, &[]).is_err());
    assert_eq!(
        fs::read(outside.join("keep.txt")).unwrap(),
        b"outside sentinel"
    );
    assert!(
        resolve(
            &app,
            &temp,
            CacheLocationMode::V2Cache,
            &link.display().to_string()
        )
        .is_err()
    );
}

#[test]
fn storage_runtime_restart_and_late_work_use_one_snapshot() {
    let (app, temp) = fixture("restart");
    let exe = std::env::current_exe().unwrap();
    for phase in ["portable", "temporary"] {
        let output = std::process::Command::new(&exe)
            .args([
                "--exact",
                "infrastructure::storage_paths::tests::storage_runtime_child",
                "--nocapture",
            ])
            .env("YT_DLP_GUI_STORAGE_TEST_APP", &app)
            .env("YT_DLP_GUI_STORAGE_TEST_TEMP", &temp)
            .env("YT_DLP_GUI_STORAGE_TEST_PHASE", phase)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(app.join("cache/audio/portable.bin").exists());
    let temporary = resolve(&app, &temp, CacheLocationMode::WindowsTemp, "cache").unwrap();
    assert!(temporary.cache_root.join("audio/temporary.bin").exists());
}

#[test]
fn storage_runtime_child() {
    let Some(app) = std::env::var_os("YT_DLP_GUI_STORAGE_TEST_APP") else {
        return;
    };
    let app = PathBuf::from(app);
    let temp = PathBuf::from(std::env::var_os("YT_DLP_GUI_STORAGE_TEST_TEMP").unwrap());
    let phase = std::env::var("YT_DLP_GUI_STORAGE_TEST_PHASE").unwrap();
    let mode = if phase == "portable" || phase.starts_with("lock-") || phase == "unavailable" {
        CacheLocationMode::V2Cache
    } else {
        CacheLocationMode::WindowsTemp
    };
    let portable = if phase.starts_with("unavailable") {
        "bad-cache"
    } else {
        "cache"
    };
    initialize_fixture(&app, &temp, mode, portable);
    if phase == "unavailable" {
        assert!(require_ready().is_err());
        assert!(use_cache().is_err());
        assert!(exclusive_cleanup().is_err());
        assert_eq!(paths().cache_root, app.join("bad-cache"));
        assert!(!paths().temporary_root.exists());
        assert!(pending_root(CacheLocationMode::WindowsTemp, portable).is_ok());
        return;
    }
    require_ready().unwrap();
    if phase == "lock-busy" {
        assert!(exclusive_cleanup().is_err());
        return;
    }
    if phase == "lock-free" {
        assert!(exclusive_cleanup().is_ok());
        return;
    }
    let active = paths().cache_root.clone();
    initialize_fixture(&app, &temp, CacheLocationMode::WindowsTemp, "cache");
    assert_eq!(paths().cache_root, active);
    assert_eq!(
        crate::infrastructure::tool_install::portable_temp_cache_dir(),
        active.join("temp")
    );
    let guard = use_cache().unwrap();
    assert!(exclusive_cleanup().is_err());
    write(&active.join(format!("audio/{phase}.bin")), b"late work");
    drop(guard);
    let cleanup = exclusive_cleanup().unwrap();
    assert!(use_cache().is_err());
    drop(cleanup);
    assert!(active.starts_with(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/cache-location-validation")
    ));
    fs::remove_dir_all(&active).unwrap();
    let _guard = use_cache().unwrap();
    write(
        &active.join(format!("audio/{phase}.bin")),
        b"recreated root",
    );
    assert_eq!(paths().cache_root, active);
}

fn run_storage_child(app: &Path, temp: &Path, phase: &str) {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "infrastructure::storage_paths::tests::storage_runtime_child",
            "--nocapture",
        ])
        .env("YT_DLP_GUI_STORAGE_TEST_APP", app)
        .env("YT_DLP_GUI_STORAGE_TEST_TEMP", temp)
        .env("YT_DLP_GUI_STORAGE_TEST_PHASE", phase)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
}

#[test]
fn storage_cleanup_is_excluded_by_another_process_using_the_cache() {
    let (app, temp) = fixture("cross-process");
    let reader = open_lock(&app.join("state/runtime/cache-use.lock")).unwrap();
    reader.try_lock_shared().unwrap();
    run_storage_child(&app, &temp, "lock-busy");
    drop(reader);
    run_storage_child(&app, &temp, "lock-free");
}

#[test]
fn storage_unavailable_root_blocks_work_without_fallback_and_can_recover_after_restart() {
    let (app, temp) = fixture("unavailable");
    write(&app.join("bad-cache"), b"not a directory");
    run_storage_child(&app, &temp, "unavailable");
    assert!(!app.join("cache").exists());
    run_storage_child(&app, &temp, "unavailable-recovery");
    assert_eq!(fs::read(app.join("bad-cache")).unwrap(), b"not a directory");
    assert!(
        resolve(&app, &temp, CacheLocationMode::WindowsTemp, "bad-cache")
            .unwrap()
            .cache_root
            .join("audio/unavailable-recovery.bin")
            .exists()
    );
}

#[cfg(windows)]
#[test]
fn storage_atomic_write_failure_keeps_the_old_document() {
    let (app, _) = fixture("atomic");
    let path = app.join("settings.yaml");
    write(&path, b"old complete document");
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();
    assert!(yaml_store::atomic_write(&path, b"new document").is_err());
    assert_eq!(fs::read(&path).unwrap(), b"old complete document");
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions).unwrap();
}

#[test]
fn storage_update_runner_uses_target_state_and_preserves_missing_stage_recovery() {
    let (app, temp) = fixture("runner");
    let paths = resolve(&app, &temp, CacheLocationMode::V2Cache, "cache").unwrap();
    let mut manifest = pending(&paths);
    manifest.backup_exe = paths
        .state_root
        .join("component-update/backup/yt-dlp-gui.old.exe");
    let record = paths
        .state_root
        .join("component-update/pending_app_update.yaml");
    write(&manifest.target_exe, b"old target fixture");
    write(&manifest.staged_exe, b"new target fixture");
    write(
        &record,
        serde_yaml::to_string(&manifest).unwrap().as_bytes(),
    );
    let args = component_update::ApplyUpdateArgs {
        parent_pid: std::process::id(),
        target_exe: manifest.target_exe.clone(),
        staged_exe: manifest.staged_exe.clone(),
        backup_exe: manifest.backup_exe.clone(),
        manifest: record.clone(),
        restart: false,
    };
    component_update::run_apply_update(args.clone()).unwrap();
    assert_eq!(
        fs::read(&manifest.target_exe).unwrap(),
        b"new target fixture"
    );
    assert_eq!(
        fs::read(&manifest.backup_exe).unwrap(),
        b"old target fixture"
    );
    assert!(
        paths
            .state_root
            .join("component-update/installed_app_version.yaml")
            .exists()
    );
    fs::remove_file(&manifest.staged_exe).unwrap();
    assert!(component_update::run_apply_update(args).is_err());
    let failed: component_update::PendingAppUpdateManifest =
        yaml_store::read_yaml_file(&record).unwrap();
    assert_eq!(
        failed.state,
        component_update::PendingAppUpdateState::Failed
    );
    assert_eq!(
        fs::read(&manifest.target_exe).unwrap(),
        b"new target fixture"
    );
    assert_eq!(
        fs::read(&manifest.backup_exe).unwrap(),
        b"old target fixture"
    );
}

#[test]
fn storage_external_output_config_accepts_relative_and_rejects_unsafe_templates() {
    let (app, _) = fixture("external-output");
    let path = app.join("yt-dlp.conf");
    for supported in [
        "-o '%(title)s.%(ext)s'\n",
        "--output=relative/%(id)s.mp4\n-P 'home:C:\\final output'\n",
        "# --output C:\\ignored.mp4\n--format best\n",
        "--output 'relative 100% literal/%(id)s.%(ext)s'\n-P 'home:final 100% literal'\n",
    ] {
        write(&path, supported.as_bytes());
        crate::infrastructure::tools::validate_external_output_config(&path).unwrap();
    }
    for unsupported in [
        "--output 'C:\\absolute output.mp4'\n",
        "-o 'subtitle:C:\\subtitles.srt'\n",
        "-o '%USERPROFILE%/file.mp4'\n",
        "-o '~/cache-review-preview/%(id)s.%(ext)s'\n",
        "--output 'thumbnail:~\\preview\\%(id)s.jpg'\n",
        "--output=subtitle:~SKY/preview/%(id)s.srt\n",
        "-o '../../outside.mp4'\n",
        "--config-locations other.conf\n",
        "--output 'unclosed\n",
    ] {
        write(&path, unsupported.as_bytes());
        assert!(
            crate::infrastructure::tools::validate_external_output_config(&path).is_err(),
            "{unsupported}"
        );
    }
}

#[test]
fn storage_cache_owner_rejects_shared_external_directory_and_rebinds_contained_copy() {
    let (app, temp) = fixture("ownership");
    let other_app = app.with_file_name("other-app");
    fs::create_dir_all(&other_app).unwrap();
    let external = app.parent().unwrap().join("shared-cache");
    let first = resolve(
        &app,
        &temp,
        CacheLocationMode::V2Cache,
        &external.display().to_string(),
    )
    .unwrap();
    ensure_cache_owner(&first).unwrap();
    let second = resolve(
        &other_app,
        &temp,
        CacheLocationMode::V2Cache,
        &external.display().to_string(),
    )
    .unwrap();
    assert!(ensure_cache_owner(&second).is_err());
    let contained = resolve(&other_app, &temp, CacheLocationMode::V2Cache, "cache").unwrap();
    write(
        &contained.cache_root.join(".yt-dlp-gui-cache.yaml"),
        &fs::read(first.cache_root.join(".yt-dlp-gui-cache.yaml")).unwrap(),
    );
    ensure_cache_owner(&contained).unwrap();
    assert!(
        fs::read_to_string(contained.cache_root.join(".yt-dlp-gui-cache.yaml"))
            .unwrap()
            .contains(&contained.installation_id)
    );
}

#[test]
fn storage_pending_update_accepts_owned_retained_temporary_root_after_temp_changes() {
    let (app, old_temp) = fixture("retained-update");
    let old = resolve(&app, &old_temp, CacheLocationMode::WindowsTemp, "cache").unwrap();
    ensure_cache_owner(&old).unwrap();
    let new_temp = old_temp.with_file_name("new-system-temp");
    fs::create_dir_all(&new_temp).unwrap();
    let current = resolve(&app, &new_temp, CacheLocationMode::WindowsTemp, "cache").unwrap();
    let mut transaction = pending(&old);
    transaction.staged_exe = old
        .cache_root
        .join("temp/manifest/staged/app/yt-dlp-gui.exe");
    transaction.backup_exe = old.state_root.join("component-update/backup/old.exe");
    write(&transaction.staged_exe, b"retained staged bytes");
    component_update::validate_pending_storage(&transaction, &current).unwrap();
    assert_ne!(old.cache_root, current.cache_root);
}
