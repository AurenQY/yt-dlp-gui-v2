//! One browser URL-analysis decision owned by V2. The host streams large item
//! records separately, then sends one logical, tool-keyed analysis result.
//! The extension assembles these frames into one cached JSON document; items
//! are intentionally never duplicated in the final frame (Native Messaging
//! limits individual messages to 1 MiB).

use super::{
    AppConfig, AtomicBool, NativeResponse, Ordering, Value, Write, host_protocol_info, json,
    stream_analysis, validated_url, write_response,
};
use crate::infrastructure::extractor_index::{
    MatchedTools, gallery_replaces_broken_yt, match_embedded_url,
};
use crate::infrastructure::gallery_dl::GalleryDlBackend;
use crate::infrastructure::operation_diagnostics;
#[path = "browser_analysis_frames.rs"]
mod frames;
use frames::{AnalysisEvent, AnalysisFrameWriter};

fn tool_result(status: &str, info: Value) -> Value {
    json!({ "status": status, "info": info, "items": [] })
}

fn overall_status(y: &str, g: &str, cancelled: bool) -> &'static str {
    if cancelled || y == "stopped" || g == "stopped" {
        return "stopped";
    }
    if ["notMatched", "disabled"].contains(&y) && ["notMatched", "disabled"].contains(&g) {
        return "notMatched";
    }
    if y == "found" || g == "found" || y == "collection" {
        return if y == "failed" || g == "failed" {
            "partial"
        } else {
            "completed"
        };
    }
    if y == "failed" || g == "failed" {
        return "failed";
    }
    "notFound"
}

fn matching_tools(url: &str, force_analysis: bool) -> Result<MatchedTools, String> {
    if force_analysis {
        Ok(MatchedTools {
            yt_dlp: true,
            gallery_dl: true,
        })
    } else {
        match_embedded_url(url)
    }
}

// A new action prevents older hosts from silently ignoring a single-tool choice.
fn allowed_tools(request: &Value) -> Result<MatchedTools, String> {
    if request["action"] != "analyzeSelected"
        && request["action"] != "analyzeSources"
        && request["action"] != "analyzeIndexedSources"
    {
        return Ok(MatchedTools {
            yt_dlp: true,
            gallery_dl: true,
        });
    }
    let tools = request["analysisTools"]
        .as_array()
        .ok_or_else(|| "analysisTools must be a nonempty list of y/g tools.".to_owned())?;
    if tools.is_empty()
        || tools.len() > 2
        || tools
            .iter()
            .any(|tool| !matches!(tool.as_str(), Some("y" | "g")))
    {
        return Err("analysisTools must be a nonempty list of y/g tools.".to_owned());
    }
    Ok(MatchedTools {
        yt_dlp: tools.iter().any(|tool| tool == "y"),
        gallery_dl: tools.iter().any(|tool| tool == "g"),
    })
}

pub(super) fn stream_unified_analysis(
    request: &Value,
    writer: &mut impl Write,
    cancelled: &AtomicBool,
) -> Result<Value, String> {
    let url = validated_url(request.get("url").unwrap_or(&Value::Null))?;
    // User intent only restricts V2's authoritative routing decision.
    let allowed = allowed_tools(request)?;
    let (config, tool_paths) = AppConfig::load_runtime();
    let mut matched = matching_tools(url, config.force_analysis)?;
    matched.yt_dlp &= allowed.yt_dlp;
    matched.gallery_dl &= allowed.gallery_dl;
    let skip_broken_yt = !config.force_analysis
        && gallery_replaces_broken_yt(
            url,
            matched,
            GalleryDlBackend::from_tool_paths(&tool_paths).is_available(),
        )?;
    if skip_broken_yt {
        matched.yt_dlp = false;
    }
    operation_diagnostics::record(
        "route.plan",
        json!({"ytSelected": matched.yt_dlp, "gallerySelected": matched.gallery_dl, "skipBrokenYt": skip_broken_yt, "forceAnalysis": config.force_analysis}),
    );
    let mut result = json!({
        "request": { "url": url, "status": "analyzing", "forceAnalysis": config.force_analysis },
        "yt-dlp": tool_result(if !allowed.yt_dlp || skip_broken_yt { "disabled" } else if matched.yt_dlp { "pending" } else { "notMatched" },
            if skip_broken_yt { json!({ "reason": "upstreamNotWorking", "working": false }) } else { json!({}) }),
        "gallery-dl": tool_result(if !allowed.gallery_dl { "disabled" } else if matched.gallery_dl { "pending" } else { "notMatched" }, json!({})),
    });
    write_response(
        &mut *writer,
        &NativeResponse {
            ok: true,
            message: "V2 selected URL analyzers",
            data: Some(json!({
                "event": "plan", "result": result, "protocol": host_protocol_info()
            })),
        },
    )?;

    std::thread::scope(|scope| -> Result<(), String> {
        // Bounded backpressure; complete length-prefixed frames cannot interleave.
        let (sender, receiver) = std::sync::mpsc::sync_channel(8);
        for (selected, backend, tool_name) in [
            (matched.yt_dlp, "y", "yt-dlp"),
            (matched.gallery_dl, "g", "gallery-dl"),
        ] {
            if !selected {
                continue;
            }
            if cancelled.load(Ordering::Acquire) {
                result[tool_name] = tool_result("stopped", json!({}));
                continue;
            }
            let mut per_tool = request.clone();
            per_tool["backend"] = json!(backend);
            let sender = sender.clone();
            let trace = operation_diagnostics::current_trace();
            scope.spawn(move || {
                let _scope = operation_diagnostics::enter(&trace);
                let mut diagnostic = operation_diagnostics::OperationSpan::start("host.analyzer", json!({"backend": backend}));
                let mut output = AnalysisFrameWriter::new(sender.clone());
                // Existing tool analysis owns cookie handling, playlistScope for Y,
                // item limits, compact item schema, and incremental item messages.
                let analysis = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    stream_analysis(&per_tool, &mut output, cancelled)
                }))
                .unwrap_or_else(|_| Err(format!("{tool_name} analysis worker failed")));
                diagnostic.finish(if cancelled.load(Ordering::Acquire) || analysis.as_ref().is_ok_and(|v| v["event"] == "stopped") { "cancelled" } else if analysis.is_ok() { "completed" } else { "failed" }, json!({"itemCount": analysis.as_ref().ok().and_then(|v| v["itemCount"].as_u64())}));
                let _ = sender.send(AnalysisEvent::Finished(backend, tool_name, analysis));
            });
        }
        drop(sender);
        for event in &receiver {
            if let AnalysisEvent::Frame(frame) = event {
                if let Err(error) = writer.write_all(&frame).and_then(|_| writer.flush()) {
                    cancelled.store(true, Ordering::Release);
                    drop(receiver);
                    return Err(format!("Could not write analysis frame: {error}"));
                }
                continue;
            }
            let AnalysisEvent::Finished(backend, tool_name, analysis) = event else {
                unreachable!()
            };
            match analysis {
                Ok(info) if info["event"] == "stopped" => {
                    result[tool_name] = tool_result("stopped", json!({}));
                }
                Ok(info) => {
                    let count = info["itemCount"].as_u64().unwrap_or(0);
                    result[tool_name] = tool_result(
                        if info["playlistRequired"] == true {
                            "collection"
                        } else if count > 0 {
                            "found"
                        } else {
                            "notFound"
                        },
                        info,
                    );
                }
                Err(message) => {
                    result[tool_name] = tool_result("failed", json!({ "message": message }));
                }
            }
            if let Err(error) = write_response(
                &mut *writer,
                &NativeResponse {
                    ok: true,
                    message: "Analyzer complete",
                    data: Some(
                        json!({ "event": "backendResult", "backend": backend, "result": result[tool_name] }),
                    ),
                },
            ) {
                cancelled.store(true, Ordering::Release);
                drop(receiver);
                return Err(error);
            }
        }
        Ok(())
    })?;
    let y = result["yt-dlp"]["status"].as_str().unwrap_or("failed");
    let g = result["gallery-dl"]["status"].as_str().unwrap_or("failed");
    let status = overall_status(y, g, cancelled.load(Ordering::Acquire));
    result["request"]["status"] = json!(status);
    // 'items' in each tool were already delivered as incremental item frames;
    // the background appends them into the corresponding group of this document.
    Ok(json!({ "event": "result", "result": result }))
}
