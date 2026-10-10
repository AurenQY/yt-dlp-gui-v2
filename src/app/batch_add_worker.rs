use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use crate::app::metadata::{
    PlaylistEntrySeed, playlist_entry_seed_from_json, select_largest_thumbnail_url,
};
use crate::infrastructure::{ToolPaths, configure_background_command, track_child_process};

pub(super) enum BatchAddEvent {
    ToolCommandFinished {
        action_id: u64,
        command_line: String,
        success: bool,
        partial_success: bool,
        detail: Option<String>,
    },
    ItemAdded {
        source: String,
        seed: PlaylistEntrySeed,
    },
    Finished {
        source: String,
        added: usize,
        stopped_by_limit: bool,
        partial_success: bool,
    },
    Failed {
        error: String,
    },
    Cancelled {
        added: usize,
    },
}

pub(super) fn run_batch_add_worker(
    tool_paths: ToolPaths,
    source: String,
    use_cookies: bool,
    limit: Option<usize>,
    untitled_task: String,
    imported_template: String,
    prefer_largest_thumbnail: bool,
    tool_log_action_id: u64,
    tx: Sender<BatchAddEvent>,
    child_handle: Arc<Mutex<Option<Child>>>,
    cancel_requested: Arc<AtomicBool>,
) {
    let mut command = match tool_paths.prepare_batch_add_command(&source, use_cookies) {
        Ok(command) => command,
        Err(error) => {
            let _ = tx.send(BatchAddEvent::ToolCommandFinished {
                action_id: tool_log_action_id,
                command_line: "yt-dlp batch import".to_owned(),
                success: false,
                partial_success: false,
                detail: Some(error.clone()),
            });
            let _ = tx.send(BatchAddEvent::Failed { error });
            return;
        }
    };
    let command_line = format_process_command_line(&command);

    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let _ = tx.send(BatchAddEvent::ToolCommandFinished {
                action_id: tool_log_action_id,
                command_line: command_line.clone(),
                success: false,
                partial_success: false,
                detail: Some(format!("Could not start yt-dlp: {error}")),
            });
            let _ = tx.send(BatchAddEvent::Failed {
                error: format!("Could not start yt-dlp: {error}"),
            });
            return;
        }
    };

    let _process_guard = track_child_process(&child, "yt-dlp batch add");

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            terminate_child_process(&mut child);
            let _ = child.wait();
            let _ = tx.send(BatchAddEvent::ToolCommandFinished {
                action_id: tool_log_action_id,
                command_line: command_line.clone(),
                success: false,
                partial_success: false,
                detail: Some("Could not read yt-dlp playlist output.".to_owned()),
            });
            let _ = tx.send(BatchAddEvent::Failed {
                error: "Could not read yt-dlp playlist output.".to_owned(),
            });
            return;
        }
    };
    let mut stderr = child.stderr.take();

    if let Ok(mut guard) = child_handle.lock() {
        *guard = Some(child);
    }

    let mut reader = BufReader::new(stdout);
    let mut added = 0usize;
    let mut buffer = String::new();
    let mut stopped_by_limit = false;

    loop {
        if cancel_requested.load(Ordering::Relaxed) {
            break;
        }

        buffer.clear();
        let bytes_read = match reader.read_line(&mut buffer) {
            Ok(bytes) => bytes,
            Err(error) => {
                let _ = tx.send(BatchAddEvent::ToolCommandFinished {
                    action_id: tool_log_action_id,
                    command_line: command_line.clone(),
                    success: false,
                    partial_success: false,
                    detail: Some(format!("Could not read yt-dlp playlist output: {error}")),
                });
                let _ = tx.send(BatchAddEvent::Failed {
                    error: format!("Could not read yt-dlp playlist output: {error}"),
                });
                clear_batch_add_child(&child_handle);
                return;
            }
        };
        if bytes_read == 0 {
            break;
        }

        if cancel_requested.load(Ordering::Relaxed) {
            break;
        }

        let line = buffer.trim();
        if line.is_empty() {
            continue;
        }

        let Ok(entry) = serde_json::from_str::<Value>(line) else {
            continue;
        };

        let Some(mut seed) =
            playlist_entry_seed_from_json(&entry, &untitled_task, &imported_template)
        else {
            continue;
        };
        if prefer_largest_thumbnail {
            if let Some(thumbnail_url) = select_largest_thumbnail_url(&entry) {
                seed.thumbnail_url = thumbnail_url;
                seed.thumbnail_hint = "Thumbnail preview".to_owned();
            }
        }

        if cancel_requested.load(Ordering::Relaxed) {
            break;
        }

        added += 1;
        let _ = tx.send(BatchAddEvent::ItemAdded {
            source: source.clone(),
            seed,
        });

        if limit.is_some_and(|value| added >= value) {
            stopped_by_limit = true;
            request_batch_add_stop(&child_handle);
            break;
        }
    }

    if cancel_requested.load(Ordering::Relaxed) {
        request_batch_add_stop(&child_handle);
    }

    let status =
        take_and_wait_batch_add_child(&child_handle, cancel_requested.load(Ordering::Relaxed));
    let was_cancelled = cancel_requested.load(Ordering::Relaxed);
    let mut command_detail = None;

    if let Some(mut stderr_reader) = stderr.take() {
        let mut stderr_text = String::new();
        let _ = stderr_reader.read_to_string(&mut stderr_text);
        if let Some(status) = status.as_ref() {
            if !status.success() && !was_cancelled && !stopped_by_limit {
                command_detail = Some(batch_import_error_summary(&stderr_text, status.code()));
            }
            if !status.success() && !was_cancelled && !stopped_by_limit && added == 0 {
                let detail = stderr_text.trim();
                let error = if detail.is_empty() {
                    format!("yt-dlp batch import failed: exit code {:?}", status.code())
                } else {
                    format!("yt-dlp batch import failed: {detail}")
                };
                let _ = tx.send(BatchAddEvent::ToolCommandFinished {
                    action_id: tool_log_action_id,
                    command_line: command_line.clone(),
                    success: false,
                    partial_success: false,
                    detail: command_detail,
                });
                let _ = tx.send(BatchAddEvent::Failed { error });
                return;
            }
        }
    }

    let command_succeeded = !was_cancelled
        && (stopped_by_limit || status.as_ref().is_some_and(|status| status.success()));
    let partial_success = !was_cancelled
        && !stopped_by_limit
        && added > 0
        && status.as_ref().is_some_and(|status| !status.success());
    let _ = tx.send(BatchAddEvent::ToolCommandFinished {
        action_id: tool_log_action_id,
        command_line: command_line.clone(),
        success: command_succeeded,
        partial_success,
        detail: command_detail,
    });

    let event = if was_cancelled {
        let _ = source;
        BatchAddEvent::Cancelled { added }
    } else {
        BatchAddEvent::Finished {
            source,
            added,
            stopped_by_limit,
            partial_success,
        }
    };
    let _ = tx.send(event);
}

fn batch_import_error_summary(stderr: &str, exit_code: Option<i32>) -> String {
    let errors = stderr
        .lines()
        .filter(|line| line.contains("ERROR:"))
        .take(3)
        .collect::<Vec<_>>()
        .join("\n");
    if !errors.is_empty() {
        errors
    } else if !stderr.trim().is_empty() {
        stderr.trim().chars().take(1200).collect()
    } else {
        format!("yt-dlp exited with code {exit_code:?}.")
    }
}

fn format_process_command_line(command: &std::process::Command) -> String {
    let program = quote_command_arg(&command.get_program().to_string_lossy());
    let args = command
        .get_args()
        .map(|arg| quote_command_arg(&arg.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ");
    if args.is_empty() {
        program
    } else {
        format!("{program} {args}")
    }
}

fn quote_command_arg(value: &str) -> String {
    if value.contains([' ', '\t', '"']) {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_owned()
    }
}

fn clear_batch_add_child(child_handle: &Arc<Mutex<Option<Child>>>) {
    if let Ok(mut guard) = child_handle.lock() {
        *guard = None;
    }
}

pub(super) fn request_batch_add_stop(child_handle: &Arc<Mutex<Option<Child>>>) {
    if let Ok(mut guard) = child_handle.lock() {
        if let Some(child) = guard.as_mut() {
            terminate_child_process(child);
        }
    }
}

fn take_and_wait_batch_add_child(
    child_handle: &Arc<Mutex<Option<Child>>>,
    force_stop: bool,
) -> Option<std::process::ExitStatus> {
    let mut child = child_handle.lock().ok()?.take()?;
    if force_stop {
        terminate_child_process(&mut child);
        for _ in 0..40 {
            if let Ok(Some(status)) = child.try_wait() {
                return Some(status);
            }
            thread::sleep(Duration::from_millis(50));
        }
        terminate_child_process(&mut child);
    }
    child.wait().ok()
}

#[cfg(target_os = "windows")]
pub(super) fn terminate_child_process(child: &mut Child) {
    let mut command = std::process::Command::new("taskkill");
    configure_background_command(&mut command);
    let _ = command
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = child.kill();
}

#[cfg(not(target_os = "windows"))]
pub(super) fn terminate_child_process(child: &mut Child) {
    let _ = child.kill();
}
