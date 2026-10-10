//! Bridge analyzer worker phases into the existing English tool-log viewer.
use super::gallery_queue_actions::{
    GalleryRouteOutcome, GalleryRoutePromptKind, GalleryRouteResult,
};
use super::*;

pub(super) fn send_analysis_log(
    tx: &Sender<GalleryRouteResult>,
    source: &str,
    video_source: &str,
    kind: GalleryRoutePromptKind,
    action_id: u64,
    tool: &'static str,
    status: ToolLogStatus,
    command: String,
    detail: String,
) {
    let _ = tx.send(GalleryRouteResult {
        source: source.to_owned(),
        video_source: video_source.to_owned(),
        kind,
        outcome: GalleryRouteOutcome::Log {
            action_id,
            tool,
            status,
            command,
            detail,
        },
    });
}
impl AppState {
    pub(super) fn record_analysis_tool_log(
        &mut self,
        action_id: u64,
        tool: &str,
        status: ToolLogStatus,
        command: String,
        detail: String,
    ) {
        if status != ToolLogStatus::Running {
            if let Some(parent) = self
                .logs
                .tool_logs
                .iter_mut()
                .find(|entry| entry.id == action_id)
            {
                if let Some(step) = parent
                    .steps
                    .iter_mut()
                    .rev()
                    .find(|step| step.tool == tool && step.status == ToolLogStatus::Running)
                {
                    step.status = status;
                    step.command = command;
                    step.detail = Some(detail);
                    if status == ToolLogStatus::Failed {
                        self.logs.viewer_expanded_action = Some(action_id);
                        self.logs.viewer_selected_step = Some(step.id);
                    }
                    parent.status = super::log::aggregate_tool_log_status(&parent.steps);
                    if status == ToolLogStatus::Failed {
                        self.reveal_log_tab_for_tool_failure();
                    }
                    return;
                }
            }
        }
        self.push_tool_log_step_with_detail_without_failure_reveal(
            action_id,
            status,
            tool,
            "analyze",
            command,
            Some(detail),
        );
        if status == ToolLogStatus::Failed {
            self.reveal_log_tab_for_tool_failure();
        }
    }
}

pub(super) fn analysis_worker_panic(
    tx: &Sender<GalleryRouteResult>,
    source: &str,
    video_source: &str,
    kind: GalleryRoutePromptKind,
    action_id: u64,
    tool: &'static str,
) -> String {
    let error = format!("{tool} analysis worker failed.");
    crate::infrastructure::operation_diagnostics::record(
        "analyzer.panic",
        serde_json::json!({"tool": tool}),
    );
    send_analysis_log(
        tx,
        source,
        video_source,
        kind,
        action_id,
        tool,
        ToolLogStatus::Failed,
        String::new(),
        format!(
            "traceId={}; reason=worker-panic",
            crate::infrastructure::operation_diagnostics::current_trace()
        ),
    );
    error
}
