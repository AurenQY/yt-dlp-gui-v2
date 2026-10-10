//! Offline input classification, independent of Add/analyze/download actions.
use std::sync::mpsc::{self, Receiver, Sender};

use crate::infrastructure::DependencyTool;
use crate::infrastructure::extractor_index::{
    YtSourceScope, matching_gallery_extractor, matching_yt_extractor_metadata,
    matching_yt_source_scope,
};
use eframe::egui::Context;

#[derive(Clone, Default, PartialEq, Eq)]
struct PreviewRequest {
    input: String,
    gallery_enabled: bool,
}

struct PreviewResult {
    scope: YtSourceScope,
    yt_working: Option<bool>,
    gallery_supported: bool,
}

#[derive(Default)]
pub(crate) struct UrlSourcePreview {
    request: PreviewRequest,
    pub(crate) scope: YtSourceScope,
    pub(crate) yt_working: Option<bool>,
    pub(crate) gallery_supported: bool,
    worker: Option<(
        Sender<PreviewRequest>,
        Receiver<(PreviewRequest, PreviewResult)>,
    )>,
}

impl UrlSourcePreview {
    pub(crate) fn update(&mut self, input: &str, gallery_enabled: bool, ctx: &Context) {
        let request = PreviewRequest {
            input: input.trim().to_owned(),
            gallery_enabled,
        };
        if self.request != request {
            let url_changed = self.request.input != request.input;
            self.request = request.clone();
            // Never keep the previous URL's answer while the new one is pending.
            // Dependency changes must not clear an already classified yt-dlp URL.
            if url_changed {
                self.scope = YtSourceScope::Unknown;
                self.yt_working = None;
            }
            self.gallery_supported = false;
            if !request.input.is_empty() {
                let (requests, _) = self.worker.get_or_insert_with(|| {
                    let (tx, rx) = mpsc::channel::<PreviewRequest>();
                    let (results, responses) = mpsc::channel();
                    let ctx = ctx.clone();
                    std::thread::spawn(move || {
                        while let Ok(mut request) = rx.recv() {
                            // Coalesce typing/paste bursts; compile/match only off UI.
                            for newer in rx.try_iter() {
                                request = newer;
                            }
                            let scope = matching_yt_source_scope(&request.input)
                                .unwrap_or(YtSourceScope::Unknown);
                            let yt_working = matching_yt_extractor_metadata(&request.input)
                                .ok()
                                .flatten()
                                .and_then(|record| record.working);
                            let gallery_supported = request.gallery_enabled
                                && matching_gallery_extractor(&request.input)
                                    .ok()
                                    .flatten()
                                    .is_some();
                            if results
                                .send((
                                    request,
                                    PreviewResult {
                                        scope,
                                        yt_working,
                                        gallery_supported,
                                    },
                                ))
                                .is_err()
                            {
                                break;
                            }
                            ctx.request_repaint();
                        }
                    });
                    (tx, responses)
                });
                let _ = requests.send(request);
            }
        }
        if let Some((_, responses)) = &self.worker {
            for (request, result) in responses.try_iter() {
                if request == self.request {
                    self.scope = result.scope;
                    self.yt_working = result.yt_working;
                    self.gallery_supported = result.gallery_supported;
                }
            }
        }
    }
}

impl super::AppState {
    /// Cached dependency presence only: no filesystem probe or process launch while typing.
    pub(crate) fn refresh_url_source_preview(&mut self, ctx: &Context) {
        let gallery_enabled = matches!(
            self.app_mode(),
            super::AppMode::Standard | super::AppMode::Origin
        ) && !self.queue_display_mode_is_audio()
            && self.prepare_report.requirements.iter().any(|requirement| {
                requirement.has_install_action(DependencyTool::GalleryDl)
                    && requirement.status.is_ok()
            });
        self.url_source_preview
            .update(&self.url_input, gallery_enabled, ctx);
    }
}
