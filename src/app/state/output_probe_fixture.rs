//! Standalone controlled probe executable; never included in normal builds.
use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::Duration,
};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--pipe-owner-child") {
        if env::var("OUTPUT_PROBE_EARLY_EXIT").unwrap() != "natural" {
            let root = PathBuf::from(env::var_os("OUTPUT_PROBE_ROOT").unwrap());
            // A finite controlled drain, released by the application-entry test.
            // No signal/termination is needed for the descendant to finish.
            for _ in 0..100 {
                println!("inherited stdout {}", "x".repeat(4096));
                eprintln!("inherited stderr {}", "x".repeat(4096));
                if root.join("finish-descendant.txt").exists() {
                    fs::write(root.join("descendant-finished.txt"), b"natural completion").unwrap();
                    return;
                }
                thread::sleep(Duration::from_millis(100));
            }
            return;
        }
        thread::sleep(Duration::from_millis(700));
        if args.iter().any(|arg| arg == "--verify-pipe-capture") {
            for _ in 0..64 {
                io::stdout().write_all(&[b'o'; 4096]).unwrap();
                io::stderr().write_all(&[b'e'; 4096]).unwrap();
            }
            print!("stdout tail drained");
            eprint!("stderr tail drained");
            return;
        }
        let tool = env::current_exe().unwrap();
        delegate(
            if tool.file_stem().unwrap() == "ffprobe" {
                "OUTPUT_PROBE_REAL_FFPROBE"
            } else {
                "OUTPUT_PROBE_REAL_FFMPEG"
            },
            &args[1..],
        );
    }
    if args.first().is_some_and(|arg| arg == "--pipe-child") {
        for _ in 0..300 {
            println!("child stdout {}", "x".repeat(4096));
            eprintln!("child stderr {}", "x".repeat(4096));
            thread::sleep(Duration::from_millis(100));
        }
        return;
    }
    let root = PathBuf::from(env::var_os("OUTPUT_PROBE_ROOT").unwrap());
    let tool = env::current_exe().unwrap();
    let name = tool.file_stem().unwrap().to_string_lossy();
    if name == "yt-dlp" {
        if args.iter().any(|arg| arg == "--dump-single-json") {
            delegate("OUTPUT_PROBE_REAL_YTDLP", &args);
        }
        let home = args
            .windows(2)
            .find_map(|pair| {
                (pair[0] == "-P")
                    .then(|| pair[1].strip_prefix("home:"))
                    .flatten()
            })
            .unwrap();
        let home = Path::new(home);
        assert!(home.starts_with(env::var_os("OUTPUT_PROBE_CACHE").unwrap()));
        let source = PathBuf::from(env::var_os("OUTPUT_PROBE_MEDIA").unwrap());
        let output = home.join(format!(
            "output.{}",
            source.extension().unwrap().to_string_lossy()
        ));
        fs::copy(source, &output).unwrap();
        append(&root, "controlled online producer");
        println!(
            "__YTDLPGUI_FINAL_PATH__=\"{}\"",
            output
                .to_string_lossy()
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
        );
        return;
    }
    let stage = if args.iter().any(|arg| arg == "--verify-pipe-capture") {
        "capture"
    } else if name == "ffprobe" {
        "ffprobe"
    } else if args.iter().any(|arg| arg == "-encoders") {
        "encoders"
    } else {
        "convert"
    };
    append(&root, stage);
    if env::var("OUTPUT_PROBE_ORPHAN_STAGE").as_deref() == Ok(stage) {
        let child = Command::new(&tool)
            .arg("--pipe-owner-child")
            .args(&args)
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        fs::write(
            root.join("active-pids.txt"),
            format!("{}\n{}\n", std::process::id(), child.id()),
        )
        .unwrap();
        io::stdout().flush().unwrap();
        io::stderr().flush().unwrap();
        std::process::exit(
            if env::var("OUTPUT_PROBE_EARLY_EXIT").unwrap() == "failure" {
                7
            } else {
                0
            },
        );
    }
    if env::var("OUTPUT_PROBE_BLOCK").unwrap() == stage {
        let child = Command::new(&tool)
            .arg("--pipe-child")
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        fs::write(
            root.join("active-pids.txt"),
            format!("{}\n{}\n", std::process::id(), child.id()),
        )
        .unwrap();
        io::stdout().flush().unwrap();
        io::stderr().flush().unwrap();
        thread::sleep(Duration::from_secs(30));
        panic!("slow probe should have been cancelled");
    }
    if stage == "encoders" && env::var("OUTPUT_PROBE_UNAVAILABLE").unwrap() == stage {
        eprintln!("Controlled encoder probe unavailable");
        std::process::exit(2);
    }
    delegate(
        if stage == "ffprobe" {
            "OUTPUT_PROBE_REAL_FFPROBE"
        } else {
            "OUTPUT_PROBE_REAL_FFMPEG"
        },
        &args,
    );
}

fn append(root: &Path, stage: &str) {
    writeln!(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("stages.log"))
            .unwrap(),
        "{stage}"
    )
    .unwrap();
}

fn delegate(key: &str, args: &[String]) -> ! {
    let status = Command::new(env::var_os(key).unwrap())
        .args(args)
        .status()
        .unwrap();
    std::process::exit(status.code().unwrap_or(1));
}
