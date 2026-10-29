//! Real filesystem/process contracts. Every fixture remains below target/.
use super::super::tools::CacheLocationMode;
use super::*;
use std::{
    process::Command,
    sync::{Arc, mpsc},
    time::{SystemTime, UNIX_EPOCH},
};

const CHILD: &str = "infrastructure::output_transaction::tests::output_filesystem_child";

#[test]
fn output_filesystem_contracts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/output-validation")
        .join(format!(
            "filesystem-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
    for mode in ["portable", "temporary"] {
        let run = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD, "--nocapture"])
            .env("OUTPUT_FS_ROOT", root.join(mode))
            .output()
            .unwrap();
        println!("{}", String::from_utf8_lossy(&run.stdout));
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
    println!("Output filesystem evidence: {}", root.display());
}

fn fixture_transaction(
    destination: &Path,
    stem: &str,
    policy: ExistingFilePolicy,
    cancel: &AtomicBool,
) -> Result<OutputTransaction, String> {
    let artifacts = OutputArtifacts::file("mp4")
        .with_suffix("en.srt")
        .with_suffix("jpg")
        .with_suffix("bin");
    OutputTransaction::new(destination, stem, artifacts, policy, cancel)
}

fn staged(owner: &OutputTransaction, suffix: &str, bytes: &[u8]) -> PathBuf {
    let path = owner.work.join(format!("output.{suffix}"));
    fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn output_filesystem_child() {
    let Some(root) = std::env::var_os("OUTPUT_FS_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let app = std::env::var_os("OUTPUT_FS_APP")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("app"));
    let temp = root.join("system-temp");
    fs::create_dir_all(&app).unwrap();
    fs::create_dir_all(&temp).unwrap();
    let mode = if root.file_name().unwrap() == "temporary" {
        CacheLocationMode::WindowsTemp
    } else {
        CacheLocationMode::V2Cache
    };
    storage_paths::initialize_fixture(&app, &temp, mode, "cache");
    let final_dir = root.join("final output 100% 空间");
    let cancel = Arc::new(AtomicBool::new(false));
    if std::env::var_os("OUTPUT_FS_CONTENDER").is_some() {
        let mut owner = fixture_transaction(
            &final_dir,
            "Concurrent",
            ExistingFilePolicy::KeepBoth,
            &cancel,
        )
        .unwrap();
        let source = staged(&owner, "bin", b"other installation");
        owner.publish(&source, &cancel).unwrap();
        println!("Second installation published {}", owner.stem);
        return;
    }
    let mut first = fixture_transaction(
        &final_dir,
        "Same title",
        ExistingFilePolicy::KeepBoth,
        &cancel,
    )
    .unwrap();
    let media = staged(&first, "mp4", b"first complete media");
    staged(&first, "f137.mp4", b"unmerged format scratch");
    staged(&first, "temp.mp4", b"incomplete processing scratch");
    staged(&first, "en.srt", b"first subtitle");
    assert!(storage_paths::exclusive_cleanup().is_err());
    first.publish(&media, &cancel).unwrap();
    drop(first);
    assert!(!final_dir.join("Same title.f137.mp4").exists());
    assert!(!final_dir.join("Same title.temp.mp4").exists());
    let mut next = fixture_transaction(
        &final_dir,
        "Same title",
        ExistingFilePolicy::KeepBoth,
        &cancel,
    )
    .unwrap();
    assert_eq!(next.stem, "Same title (2)");
    let media = staged(&next, "mp4", b"second complete media");
    staged(&next, "en.srt", b"second subtitle");
    next.publish(&media, &cancel).unwrap();
    drop(next);
    assert_eq!(
        fs::read(final_dir.join("Same title.mp4")).unwrap(),
        b"first complete media"
    );
    assert_eq!(
        fs::read(final_dir.join("Same title (2).en.srt")).unwrap(),
        b"second subtitle"
    );
    fs::write(final_dir.join("Sidecar.en.srt"), b"old sidecar only").unwrap();
    let skip =
        fixture_transaction(&final_dir, "Sidecar", ExistingFilePolicy::Skip, &cancel).unwrap();
    assert_eq!(
        skip.skipped.as_ref().unwrap().file_name().unwrap(),
        "Sidecar.en.srt"
    );
    drop(skip);

    // Distinct installations use one stable destination lock namespace.
    let mut held = fixture_transaction(
        &final_dir,
        "Concurrent",
        ExistingFilePolicy::KeepBoth,
        &cancel,
    )
    .unwrap();
    let source = staged(&held, "bin", b"first installation");
    let run = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD, "--nocapture"])
        .env("OUTPUT_FS_ROOT", &root)
        .env("OUTPUT_FS_APP", root.join("another-app"))
        .env("OUTPUT_FS_CONTENDER", "1")
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    held.publish(&source, &cancel).unwrap();
    drop(held);
    assert_eq!(
        fs::read(final_dir.join("Concurrent.bin")).unwrap(),
        b"first installation"
    );
    assert_eq!(
        fs::read(final_dir.join("Concurrent (2).bin")).unwrap(),
        b"other installation"
    );

    // Skip waits for a live lock and observes publication after its release.
    let mut held =
        fixture_transaction(&final_dir, "Waiting", ExistingFilePolicy::KeepBoth, &cancel).unwrap();
    let source = staged(&held, "bin", b"completed while waiting");
    let (tx, rx) = mpsc::channel();
    let destination = final_dir.clone();
    let stopping = cancel.clone();
    let waiter = std::thread::spawn(move || {
        tx.send(fixture_transaction(
            &destination,
            "Waiting",
            ExistingFilePolicy::Skip,
            &stopping,
        ))
        .unwrap()
    });
    assert!(rx.recv_timeout(Duration::from_millis(120)).is_err());
    held.publish(&source, &cancel).unwrap();
    drop(held);
    assert!(
        rx.recv_timeout(Duration::from_secs(3))
            .unwrap()
            .unwrap()
            .skipped
            .is_some()
    );
    waiter.join().unwrap();
    let held = fixture_transaction(
        &final_dir,
        "Cancelled wait",
        ExistingFilePolicy::KeepBoth,
        &cancel,
    )
    .unwrap();
    let stopping = Arc::new(AtomicBool::new(false));
    let wait_cancel = stopping.clone();
    let destination = final_dir.clone();
    let waiter = std::thread::spawn(move || {
        fixture_transaction(
            &destination,
            "Cancelled wait",
            ExistingFilePolicy::Skip,
            &wait_cancel,
        )
    });
    std::thread::sleep(Duration::from_millis(100));
    stopping.store(true, Ordering::Release);
    assert!(waiter.join().unwrap().is_err());
    drop(held);
    let held = fixture_transaction(
        &final_dir,
        "Failed producer",
        ExistingFilePolicy::KeepBoth,
        &cancel,
    )
    .unwrap();
    let destination = final_dir.clone();
    let stopping = cancel.clone();
    let waiter = std::thread::spawn(move || {
        fixture_transaction(
            &destination,
            "Failed producer",
            ExistingFilePolicy::Skip,
            &stopping,
        )
    });
    std::thread::sleep(Duration::from_millis(80));
    drop(held);
    assert!(waiter.join().unwrap().unwrap().skipped.is_none());
    #[cfg(windows)]
    {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetShortPathNameW(long: *const u16, short: *mut u16, length: u32) -> u32;
        }
        let long: Vec<u16> = final_dir.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut buffer = vec![0u16; 32768];
        let length =
            unsafe { GetShortPathNameW(long.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
        assert!(length > 0 && length < buffer.len() as u32);
        let alias = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length as usize]));
        assert_ne!(
            alias.to_string_lossy().to_lowercase(),
            final_dir.to_string_lossy().to_lowercase()
        );
        let held = fixture_transaction(&final_dir, "Alias", ExistingFilePolicy::KeepBoth, &cancel)
            .unwrap();
        let via_alias =
            fixture_transaction(&alias, "ALIAS", ExistingFilePolicy::KeepBoth, &cancel).unwrap();
        assert!(via_alias.stem.ends_with(" (2)"));
        drop(via_alias);
        drop(held);
        use std::os::windows::fs::OpenOptionsExt;
        let mut blocked =
            fixture_transaction(&final_dir, "Blocked", ExistingFilePolicy::KeepBoth, &cancel)
                .unwrap();
        let source = staged(&blocked, "bin", b"complete staged file");
        let lock = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&source)
            .unwrap();
        assert!(
            blocked
                .publish(&source, &cancel)
                .unwrap_err()
                .to_string()
                .contains("publication failed")
        );
        assert!(!final_dir.join("Blocked.bin").exists());
        drop(lock);
        drop(blocked);
    }

    // A non-cooperating writer wins a late race; the entire new group moves.
    let mut race =
        fixture_transaction(&final_dir, "Race", ExistingFilePolicy::KeepBoth, &cancel).unwrap();
    let source = staged(&race, "mp4", b"new group");
    staged(&race, "jpg", b"new cover");
    fs::write(final_dir.join("Race.jpg"), b"external cover").unwrap();
    race.publish(&source, &cancel).unwrap();
    assert_eq!(race.stem, "Race (2)");
    drop(race);
    assert_eq!(
        fs::read(final_dir.join("Race.jpg")).unwrap(),
        b"external cover"
    );

    // Deterministic injection between two actual non-replacing filesystem commits.
    let mut partial =
        fixture_transaction(&final_dir, "Partial", ExistingFilePolicy::KeepBoth, &cancel).unwrap();
    let source = staged(&partial, "mp4", b"complete primary");
    staged(&partial, "jpg", b"new sidecar");
    let conflict = final_dir.join("Partial.jpg");
    partial.after_publish = Some(Box::new(move |_| {
        if !conflict.exists() {
            fs::write(&conflict, b"late external sidecar").unwrap();
        }
    }));
    let error = partial.publish(&source, &cancel).unwrap_err();
    assert_eq!(
        error.disposition,
        super::super::output_policy::OutputDisposition::PartialFailure
    );
    assert_eq!(error.published_paths, partial.published);
    assert_eq!(
        partial.published,
        vec![final_dir.join("Partial.mp4").canonicalize().unwrap()]
    );
    drop(partial);
    assert_eq!(
        fs::read(final_dir.join("Partial.mp4")).unwrap(),
        b"complete primary"
    );
    assert_eq!(
        fs::read(final_dir.join("Partial.jpg")).unwrap(),
        b"late external sidecar"
    );

    let mut chapter = OutputTransaction::new(
        &final_dir,
        "Chapters",
        OutputArtifacts::directory(),
        ExistingFilePolicy::KeepBoth,
        &cancel,
    )
    .unwrap();
    let directory = chapter.work.join("output");
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("01 - Intro.mp3"), b"chapter").unwrap();
    chapter.publish(&directory, &cancel).unwrap();
    drop(chapter);
    assert!(final_dir.join("Chapters/01 - Intro.mp3").is_file());
    assert!(
        OutputTransaction::new(
            &final_dir,
            "Chapters",
            OutputArtifacts::directory(),
            ExistingFilePolicy::Skip,
            &cancel
        )
        .unwrap()
        .skipped
        .is_some()
    );

    fs::write(final_dir.join("Exact.mp4"), b"conversion source").unwrap();
    fs::write(final_dir.join("Exact.txt"), b"unrelated same-stem neighbor").unwrap();
    let mut exact = OutputTransaction::new(
        &final_dir,
        "Exact",
        OutputArtifacts::file("mkv").with_suffix("en.srt"),
        ExistingFilePolicy::Skip,
        &cancel,
    )
    .unwrap();
    assert!(exact.skipped.is_none());
    let primary = staged(&exact, "mkv", b"requested output");
    exact.publish(&primary, &cancel).unwrap();
    drop(exact);
    assert_eq!(
        fs::read(final_dir.join("Exact.mp4")).unwrap(),
        b"conversion source"
    );
    assert_eq!(
        fs::read(final_dir.join("Exact.txt")).unwrap(),
        b"unrelated same-stem neighbor"
    );
    fs::write(
        final_dir.join("Requested.en.srt"),
        b"requested sidecar already present",
    )
    .unwrap();
    let exact = OutputTransaction::new(
        &final_dir,
        "Requested",
        OutputArtifacts::file("mkv").with_suffix("en.srt"),
        ExistingFilePolicy::Skip,
        &cancel,
    )
    .unwrap();
    assert_eq!(
        exact.skipped.as_ref().unwrap().file_name().unwrap(),
        "Requested.en.srt"
    );
    drop(exact);

    // Exercise the actual copy/hash/flush/no-replace fallback without pretending
    // that a second physical volume exists in this workspace.
    let copy_source = root.join("copy-source.bin");
    let target = final_dir.join("copied.bin");
    fs::write(&copy_source, vec![42; 131_071]).unwrap();
    let before = hash_file(&copy_source).unwrap();
    publish_copy(&copy_source, &target, &cancel).unwrap();
    assert_eq!(hash_file(&target).unwrap(), before);
    fs::write(&copy_source, b"must survive collision").unwrap();
    assert!(publish_copy(&copy_source, &target, &cancel).is_err());
    assert_eq!(fs::read(&copy_source).unwrap(), b"must survive collision");
    assert_eq!(hash_file(&target).unwrap(), before);
    assert!(cross_device(&io::Error::from_raw_os_error(
        if cfg!(windows) { 17 } else { 18 }
    )));
    assert!(!cross_device(&io::Error::from_raw_os_error(5)));

    let unknown = root.join("unknown/.yt-dlp-gui-output");
    fs::create_dir_all(&unknown).unwrap();
    fs::write(unknown.join("personal.txt"), b"preserve").unwrap();
    assert!(
        fixture_transaction(
            unknown.parent().unwrap(),
            "name",
            ExistingFilePolicy::KeepBoth,
            &cancel
        )
        .is_err()
    );
    assert_eq!(fs::read(unknown.join("personal.txt")).unwrap(), b"preserve");
    let orphan = final_dir.join(".yt-dlp-gui-publish-unknown.tmp");
    fs::write(&orphan, b"unknown orphan").unwrap();
    let mut outside =
        fixture_transaction(&final_dir, "Outside", ExistingFilePolicy::KeepBoth, &cancel).unwrap();
    assert!(outside.publish(&copy_source, &cancel).is_err());
    drop(outside);
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut owner = fixture_transaction(
        &final_dir,
        "Cancelled",
        ExistingFilePolicy::KeepBoth,
        &cancelled,
    )
    .unwrap();
    let work = owner.work.clone();
    let source = staged(&owner, "bin", b"unpublished");
    cancelled.store(true, Ordering::Release);
    assert!(owner.publish(&source, &cancelled).is_err());
    drop(owner);
    assert!(!work.exists());
    let cleanup = storage_paths::exclusive_cleanup().unwrap();
    storage_paths::clear_disposable(storage_paths::paths(), &[]).unwrap();
    drop(cleanup);
    assert_eq!(
        fs::read(final_dir.join("Same title.mp4")).unwrap(),
        b"first complete media"
    );
    assert_eq!(fs::read(orphan).unwrap(), b"unknown orphan");
    println!(
        "{mode:?}: shared process locks, thread Skip wait/cancel, sidecar groups, late/partial races, chapter groups, injected cross-volume copy, unknown namespace, cleanup: passed"
    );
}
