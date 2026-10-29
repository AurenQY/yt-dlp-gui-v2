//! Standalone opt-in test executable, built by the output review fixture only.
//! Delegates to real tools except for the explicitly controlled producer/stall.
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
    if args
        .first()
        .is_some_and(|arg| arg == "--fixture-pipe-child")
    {
        for _ in 0..300 {
            println!("fixture child stdout {}", "x".repeat(4096));
            eprintln!("fixture child stderr {}", "x".repeat(4096));
            thread::sleep(Duration::from_millis(100));
        }
        return;
    }
    let root = PathBuf::from(env::var_os("OUTPUT_POSTPASS_ROOT").expect("fixture root"));
    let tool = env::current_exe().unwrap();
    if tool
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .contains("yt-dlp")
    {
        if args
            .iter()
            .any(|arg| matches!(arg.as_str(), "--dump-single-json" | "--dump-json"))
        {
            delegate("OUTPUT_POSTPASS_REAL_YTDLP", &args);
        }
        let work = args
            .windows(2)
            .find_map(|pair| {
                (pair[0] == "-P")
                    .then(|| pair[1].strip_prefix("home:"))
                    .flatten()
            })
            .expect("owned home path");
        let work = Path::new(work);
        assert!(work.starts_with(env::var_os("OUTPUT_POSTPASS_ALLOWED_CACHE").unwrap()));
        let ext = env::var("OUTPUT_POSTPASS_MEDIA_EXT").unwrap();
        let output = work.join(format!("output.{ext}"));
        fs::copy(env::var_os("OUTPUT_POSTPASS_MEDIA").unwrap(), &output).unwrap();
        append(&root, "controlled online producer");
        let quoted = output
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        println!("__YTDLPGUI_FINAL_PATH__=\"{quoted}\"");
        return;
    }
    let output = args.last().expect("FFmpeg output");
    let stage = if output.contains("cover-pass.") {
        "cover"
    } else if output.contains("metadata-pass.") {
        "metadata"
    } else {
        "extension"
    };
    append(&root, stage);
    if env::var("OUTPUT_POSTPASS_BLOCK").unwrap() == stage {
        let child = Command::new(env::current_exe().unwrap())
            .arg("--fixture-pipe-child")
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        fs::write(
            root.join("active-pids.txt"),
            format!("{}\n{}\n", std::process::id(), child.id()),
        )
        .unwrap();
        println!("fixture parent stdout active");
        eprintln!("fixture parent stderr active");
        io::stdout().flush().unwrap();
        io::stderr().flush().unwrap();
        thread::sleep(Duration::from_secs(30));
        panic!("The slow fixture must have been cancelled before its timeout");
    }
    delegate("OUTPUT_POSTPASS_REAL_FFMPEG", &args);
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
    let status = Command::new(env::var_os(key).expect("real tool path"))
        .args(args)
        .status()
        .unwrap();
    std::process::exit(status.code().unwrap_or(1));
}
