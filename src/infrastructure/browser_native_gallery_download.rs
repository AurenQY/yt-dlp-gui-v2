//! Native Host Gallery process/XMP lifecycle. Download arguments, event parsing
//! and canonical XMP mapping are shared with desktop; browser framing/ZIP is not.

use std::{
    io::{BufRead, BufReader, Read},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};

use crate::{
    app::gallery_canonical_metadata::map_gallery_artifact_metadata,
    infrastructure::{
        gallery_dl::{
            GalleryDlMetadataEvent, PreparedGalleryDlDownload, parse_gallery_dl_download_event,
        },
        gallery_xmp::write_gallery_xmp,
        track_child_process,
    },
};

pub(super) fn run_gallery_download(
    mut prepared: PreparedGalleryDlDownload,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    let mut diagnostic = crate::infrastructure::operation_diagnostics::OperationSpan::start(
        "host.gallery.download",
        serde_json::json!({"command": crate::infrastructure::operation_diagnostics::command_context(&prepared.command)}),
    );
    prepared
        .command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = prepared
        .command
        .spawn()
        .map_err(|error| format!("Could not start gallery-dl: {error}"))?;
    let _tracked = track_child_process(&child, "gallery-dl browser post download");
    // Cover cancellation between the caller's initial check and registration.
    if cancelled.load(Ordering::Acquire) {
        crate::infrastructure::force_cleanup_tracked_processes();
    }
    let stdout = child.stdout.take().expect("piped gallery-dl stdout");
    let mut stderr = child.stderr.take().expect("piped gallery-dl stderr");

    let (status, stdout_result, metadata_result, stderr_result) = thread::scope(|scope| {
        // Keep pipe reads independent of XMP writes without retaining the whole
        // gallery in memory. On errors/cancellation still drain the process pipes.
        let (tx, rx) = mpsc::sync_channel::<GalleryDlMetadataEvent>(16);
        let trace = crate::infrastructure::operation_diagnostics::current_trace();
        let metadata_worker = scope.spawn(move || {
            let _scope = crate::infrastructure::operation_diagnostics::enter(&trace);
            for event in rx {
                if cancelled.load(Ordering::Acquire) {
                    break;
                }
                let fields = map_gallery_artifact_metadata(&event.metadata, &event.artifact.path);
                if let Err(error) =
                    write_gallery_xmp(&event.artifact.path, &event.artifact.archive_key, &fields)
                {
                    // Match desktop: unsupported images are skipped by the writer;
                    // write failures warn without changing download success policy.
                    eprintln!(
                        "[gallery-dl] XMP post-processing warning for {}: {error}",
                        event.artifact.path.display()
                    );
                }
            }
        });
        let stdout_reader = scope.spawn(move || -> Result<(), String> {
            let mut reader = BufReader::new(stdout);
            let mut raw_line = Vec::new();
            let mut first_error = None;
            loop {
                raw_line.clear();
                if reader
                    .read_until(b'\n', &mut raw_line)
                    .map_err(|error| format!("Could not read gallery-dl output: {error}"))?
                    == 0
                {
                    break;
                }
                let line = String::from_utf8_lossy(&raw_line);
                match parse_gallery_dl_download_event(&line) {
                    Some(Ok(event)) => {
                        if tx.send(event).is_err()
                            && !cancelled.load(Ordering::Acquire)
                            && first_error.is_none()
                        {
                            first_error =
                                Some("gallery-dl XMP worker stopped unexpectedly.".to_owned());
                        }
                    }
                    Some(Err(error)) if first_error.is_none() => first_error = Some(error),
                    _ => {}
                }
            }
            first_error.map_or(Ok(()), Err)
        });
        let stderr_reader = scope.spawn(move || -> Result<String, String> {
            let mut tail = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let count = stderr
                    .read(&mut buffer)
                    .map_err(|error| format!("Could not read gallery-dl errors: {error}"))?;
                if count == 0 {
                    break;
                }
                tail.extend_from_slice(&buffer[..count]);
                if tail.len() > 16 * 1024 {
                    tail.drain(..tail.len() - 16 * 1024);
                }
            }
            Ok(String::from_utf8_lossy(&tail).trim().to_owned())
        });
        let status = child.wait();
        if status.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        (
            status,
            stdout_reader.join(),
            metadata_worker.join(),
            stderr_reader.join(),
        )
    });

    if cancelled.load(Ordering::Acquire) {
        diagnostic.finish("cancelled", serde_json::json!({}));
        return Ok(());
    }
    let status = status.map_err(|error| format!("Could not wait for gallery-dl: {error}"))?;
    crate::infrastructure::operation_diagnostics::record(
        "tool.exit",
        serde_json::json!({"exitCode": status.code(), "success": status.success()}),
    );
    let stderr_detail =
        stderr_result.map_err(|_| "gallery-dl error reader stopped unexpectedly.".to_owned())??;
    if !status.success() {
        let detail = if stderr_detail.is_empty() {
            format!("process exited with {status}")
        } else {
            stderr_detail.chars().take(4096).collect()
        };
        return Err(format!("gallery-dl download failed: {detail}"));
    }
    stdout_result.map_err(|_| "gallery-dl output reader stopped unexpectedly.".to_owned())??;
    metadata_result.map_err(|_| "gallery-dl XMP worker stopped unexpectedly.".to_owned())?;
    diagnostic.finish("success", serde_json::json!({"exitCode": status.code()}));
    Ok(())
}
