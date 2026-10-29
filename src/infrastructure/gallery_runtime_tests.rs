use super::*;

#[test]
fn gallery_bundle_paths_and_unapproved_identity_are_rejected() {
    for name in [
        "",
        "../escape",
        "/absolute",
        "C:/drive",
        "a\\b",
        "a//b",
        "a/./b",
    ] {
        assert!(safe_relative(name).is_err(), "{name}");
    }
    assert!(safe_relative("_internal/python312.dll").is_ok());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/cache-location-validation/untrusted-gallery-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("gallery-dl.exe"),
        b"untrusted candidate must never execute",
    )
    .unwrap();
    fs::write(
        root.join("bundle.manifest.json"),
        br#"{"schema":1,"bundle_id":"unapproved"}"#,
    )
    .unwrap();
    assert!(verify_bundle(&root).is_err());
}

#[test]
#[ignore = "Set YT_DLP_GUI_GALLERY_BUNDLE to the locally built standalone artifact"]
fn gallery_runtime_real_bundle_transactions() {
    let artifact = PathBuf::from(
        std::env::var_os("YT_DLP_GUI_GALLERY_BUNDLE").expect("Set YT_DLP_GUI_GALLERY_BUNDLE"),
    );
    verify_bundle(&artifact).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "target/cache-location-validation/gallery-runtime-{}",
        std::process::id()
    ));
    for mode in ["portable", "temporary"] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "infrastructure::gallery_runtime_policy::tests::gallery_runtime_child",
                "--nocapture",
            ])
            .env("YT_DLP_GUI_RUNTIME_TEST_APP", root.join(mode))
            .env("YT_DLP_GUI_RUNTIME_TEST_MODE", mode)
            .env("YT_DLP_GUI_GALLERY_BUNDLE", &artifact)
            .output()
            .unwrap();
        println!("{}", String::from_utf8_lossy(&output.stdout));
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    println!("Runtime transaction fixture: {}", root.display());
}

#[test]
fn gallery_runtime_child() {
    let Some(app) = std::env::var_os("YT_DLP_GUI_RUNTIME_TEST_APP") else {
        return;
    };
    let app = PathBuf::from(app);
    let temp = app.parent().unwrap().join("system-temp");
    fs::create_dir_all(&app).unwrap();
    fs::create_dir_all(&temp).unwrap();
    let mode = if std::env::var("YT_DLP_GUI_RUNTIME_TEST_MODE").unwrap() == "portable" {
        super::super::tools::CacheLocationMode::V2Cache
    } else {
        super::super::tools::CacheLocationMode::WindowsTemp
    };
    storage_paths::initialize_fixture(&app, &temp, mode, "cache");
    storage_paths::require_ready().unwrap();
    let artifact = PathBuf::from(std::env::var_os("YT_DLP_GUI_GALLERY_BUNDLE").unwrap());
    let manifest = verify_bundle(&artifact).unwrap();
    let destination = app.join("tools/gallery-dl");
    fs::create_dir_all(&destination).unwrap();
    fs::write(
        destination.join("gallery-dl.exe"),
        b"old managed executable sentinel",
    )
    .unwrap();
    fs::write(
        destination.join("personal-notes.txt"),
        b"unrelated sentinel",
    )
    .unwrap();
    assert!(install_managed().is_err());
    assert_eq!(
        fs::read(destination.join("gallery-dl.exe")).unwrap(),
        b"old managed executable sentinel"
    );
    let input = app
        .join("tool-bundles/gallery-dl")
        .join(&manifest.bundle_id);
    install::copy_tree(&artifact, &input).unwrap();
    let cancel_checks = std::cell::Cell::new(0);
    let error = install_managed_with_cancel(&|| {
        cancel_checks.set(cancel_checks.get() + 1);
        cancel_checks.get() > 2
    })
    .unwrap_err();
    assert!(error.contains("cancelled"));
    assert_eq!(
        fs::read(destination.join("gallery-dl.exe")).unwrap(),
        b"old managed executable sentinel"
    );
    assert!(
        !app.join("state/component-update/gallery-dl-install.yaml")
            .exists()
    );
    let program = install_managed().unwrap();
    assert_eq!(program, destination.join("gallery-dl.exe"));
    assert_eq!(
        fs::read(destination.join("personal-notes.txt")).unwrap(),
        b"unrelated sentinel"
    );
    let guard = acquire(&program).unwrap();
    assert!(install_managed().is_err());
    assert!(storage_paths::exclusive_cleanup().is_err());
    let competing = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "infrastructure::gallery_runtime_policy::tests::gallery_runtime_busy_child",
        ])
        .env("YT_DLP_GUI_RUNTIME_TEST_BUSY", "1")
        .output()
        .unwrap();
    assert!(
        competing.status.success(),
        "{}",
        String::from_utf8_lossy(&competing.stderr)
    );
    drop(guard);
    // Inventory tampering cannot reach the capability execution step.
    let tampered = destination.join("_internal/python312.dll");
    let original = fs::read(&tampered).unwrap();
    fs::write(&tampered, b"tampered").unwrap();
    assert!(acquire(&program).is_err());
    fs::write(&tampered, original).unwrap();
    fs::write(destination.join("unexpected.dll"), b"unknown library").unwrap();
    assert!(acquire(&program).is_err());
    fs::remove_file(destination.join("unexpected.dll")).unwrap();
    // A crash after retaining the old tree is recovered before work is admitted.
    let transaction = "123-456-789";
    let state_path = app.join("state/component-update/gallery-dl-installed.yaml");
    let previous_state = fs::read_to_string(&state_path).unwrap();
    let prepared = app.join(format!("tools/.gallery-dl-install-{transaction}"));
    let backup = app
        .join("state/component-update/gallery-dl-backup")
        .join(transaction);
    fs::create_dir_all(backup.parent().unwrap()).unwrap();
    fs::create_dir_all(&prepared).unwrap();
    fs::write(prepared.join("partial.bin"), b"partial").unwrap();
    fs::rename(&destination, &backup).unwrap();
    fs::write(
        app.join("state/component-update/gallery-dl-install.yaml"),
        serde_yaml::to_string(&serde_json::json!({"transaction_id":transaction,"phase":"retained","had_previous":true,"previous_state":previous_state})).unwrap(),
    )
    .unwrap();
    // Normal preparation itself must recover, even with the EXE still absent.
    drop(acquire(&program).unwrap());
    assert!(!prepared.exists());
    assert!(!backup.exists());
    assert_eq!(fs::read_to_string(&state_path).unwrap(), previous_state);
    readiness(&program).unwrap();
    assert_eq!(
        fs::read(destination.join("personal-notes.txt")).unwrap(),
        b"unrelated sentinel"
    );
    // An interrupted publication with an incomplete new tree restores all of
    // the previous bundle before any candidate capability can be executed.
    fs::rename(&destination, &backup).unwrap();
    fs::create_dir_all(&destination).unwrap();
    fs::write(destination.join("gallery-dl.exe"), b"incomplete new tree").unwrap();
    fs::write(
        app.join("state/component-update/gallery-dl-install.yaml"),
        serde_yaml::to_string(&serde_json::json!({"transaction_id":transaction,"phase":"published","had_previous":true,"previous_state":previous_state})).unwrap(),
    ).unwrap();
    drop(acquire(&program).unwrap());
    assert_eq!(fs::read_to_string(&state_path).unwrap(), previous_state);
    assert!(!backup.exists());
    println!(
        "Verified runtime, cancelled install, busy replacement, tampering, unknown library, and retained/published-tree recovery: {}",
        app.display()
    );
}

#[test]
fn gallery_runtime_busy_child() {
    if std::env::var_os("YT_DLP_GUI_RUNTIME_TEST_BUSY").is_none() {
        return;
    }
    let app = PathBuf::from(std::env::var_os("YT_DLP_GUI_RUNTIME_TEST_APP").unwrap());
    let temp = app.parent().unwrap().join("system-temp");
    let mode = if std::env::var("YT_DLP_GUI_RUNTIME_TEST_MODE").unwrap() == "portable" {
        super::super::tools::CacheLocationMode::V2Cache
    } else {
        super::super::tools::CacheLocationMode::WindowsTemp
    };
    storage_paths::initialize_fixture(&app, &temp, mode, "cache");
    assert!(install_managed().unwrap_err().contains("in use"));
}
