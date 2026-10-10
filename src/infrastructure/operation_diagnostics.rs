//! Fixed-English, privacy-limited operation journal shared by desktop and Host.
//! Never pass raw URLs, tool output, cookies, headers or user paths to this journal.
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    fs::{self, OpenOptions},
    io::Write,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

thread_local! { static TRACE: RefCell<Option<String>> = const { RefCell::new(None) }; }
static NEXT: AtomicU64 = AtomicU64::new(1);
const MAX_BYTES: u64 = 4 * 1024 * 1024;

pub(crate) fn new_trace() -> String {
    format!(
        "{}-{}-{}",
        std::process::id(),
        timestamp_ms(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
pub(crate) fn current_trace() -> String {
    TRACE
        .with(|value| value.borrow().clone())
        .unwrap_or_else(new_trace)
}
pub(crate) struct TraceScope(Option<String>);
pub(crate) fn enter(trace: &str) -> TraceScope {
    let trace = if trace.len() <= 80
        && !trace.is_empty()
        && trace
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        trace.to_owned()
    } else {
        new_trace()
    };
    TraceScope(TRACE.with(|value| value.replace(Some(trace))))
}
impl Drop for TraceScope {
    fn drop(&mut self) {
        TRACE.with(|value| {
            value.replace(self.0.take());
        });
    }
}

/// Bounded JSONL, two generations, cross-process lock. Logging never fails work.
pub(crate) fn record(event: &'static str, fields: Value) {
    let trace = current_trace();
    let result = (|| -> std::io::Result<()> {
        let directory = super::runtime_paths::application_root().join("data/diagnostics");
        fs::create_dir_all(&directory)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("operations.lock"))?;
        lock.lock()?;
        let path = directory.join("operations.jsonl");
        if fs::metadata(&path).is_ok_and(|m| m.len() >= MAX_BYTES) {
            // Only our own bounded journal is rotated; no user queue/cache is touched.
            fs::copy(&path, directory.join("operations.previous.jsonl"))?;
            OpenOptions::new().write(true).truncate(true).open(&path)?;
        }
        let row = json!({"timestampMs": timestamp_ms(), "pid": std::process::id(), "traceId": trace, "event": event, "fields": fields});
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{row}")?;
        file.flush()
    })();
    if result.is_err() {
        eprintln!("[operation-diagnostics] journal write unavailable");
    }
}

#[derive(Debug)]
pub(crate) struct OperationSpan {
    operation: &'static str,
    id: String,
    trace: String,
    started: Instant,
    finished: bool,
}
impl OperationSpan {
    pub(crate) fn start(operation: &'static str, fields: Value) -> Self {
        let trace = current_trace();
        let _scope = enter(&trace);
        let id = new_trace();
        record(
            "operation.start",
            json!({"operation": operation, "spanId": id, "context": fields}),
        );
        Self {
            operation,
            id,
            trace,
            started: Instant::now(),
            finished: false,
        }
    }
    pub(crate) fn finish(&mut self, status: &'static str, fields: Value) {
        if self.finished {
            return;
        }
        self.finished = true;
        let _scope = enter(&self.trace);
        record(
            "operation.finish",
            json!({"operation": self.operation, "spanId": self.id, "status": status, "elapsedMs": self.started.elapsed().as_millis(), "result": fields}),
        );
    }
}
impl Drop for OperationSpan {
    fn drop(&mut self) {
        self.finish("aborted-or-error", json!({}));
    }
}

/// Command structure without argument values; exact raw command stays in GUI only.
pub(crate) fn command_context(command: &Command) -> Value {
    let args = command
        .get_args()
        .map(|s| s.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let flags = args
        .iter()
        .filter(|s| s.starts_with('-'))
        .map(|s| s.split('=').next().unwrap_or(""))
        .filter(|s| {
            [
                "--config-ignore",
                "--ignore-config",
                "--config-locations",
                "--no-input",
                "--no-colors",
                "-J",
                "--dump-json",
                "--dump-single-json",
                "--flat-playlist",
                "--lazy-playlist",
                "--no-warnings",
                "--ignore-errors",
                "--post-range",
                "--range",
                "--playlist-items",
                "--playlist-end",
                "--no-playlist",
                "--yes-playlist",
                "--impersonate",
                "--cookies",
                "--cookies-from-browser",
                "--add-header",
                "--proxy",
                "--username",
                "--password",
                "--format",
                "-f",
                "--output",
                "-o",
                "--write-info-json",
                "--download-archive",
                "--simulate",
                "--skip-download",
            ]
            .contains(s)
        })
        .collect::<Vec<_>>();
    let cookies = if args.iter().any(|s| s == "--cookies-from-browser") {
        "browser"
    } else if args.iter().any(|s| s == "--cookies") {
        "file-or-native-override"
    } else {
        "none"
    };
    let bounds = args
        .windows(2)
        .filter(|pair| {
            [
                "--post-range",
                "--range",
                "--playlist-items",
                "--playlist-end",
            ]
            .contains(&pair[0].as_str())
                && pair[1].len() <= 80
                && pair[1]
                    .bytes()
                    .all(|b| b.is_ascii_digit() || b == b'-' || b == b',' || b == b':')
        })
        .map(|pair| (pair[0].clone(), json!(pair[1])))
        .collect::<serde_json::Map<String, Value>>();
    json!({"flags": flags, "bounds": bounds, "argumentCount": args.len(), "cookieSource": cookies})
}
