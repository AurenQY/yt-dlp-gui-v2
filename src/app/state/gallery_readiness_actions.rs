//! Display-only gallery readiness; actual operations independently validate under a lease.
use super::*;

const GALLERY_CHECKING_DETAIL: &str = "Checking gallery-dl compatibility...";

#[derive(Default)]
pub(super) struct GalleryReadinessState {
    configured_path: String,
    resolved_path: PathBuf,
    pub(super) result: Option<Result<(), String>>,
    pending: Option<Receiver<Result<(), String>>>,
    repaint: Arc<std::sync::OnceLock<eframe::egui::Context>>,
    #[cfg(any(test, cache_review_gui))]
    events: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl AppState {
    pub(crate) fn bind_gallery_repaint(&self, ctx: &eframe::egui::Context) {
        let _ = self.gallery_readiness.repaint.set(ctx.clone());
    }

    pub(super) fn request_gallery_readiness(&mut self, invalidate: bool) {
        if !invalidate
            && self.gallery_readiness.pending.is_some()
            && self.gallery_readiness.configured_path == self.tool_paths.gallery_dl
        {
            return;
        }
        let path = crate::infrastructure::resolve_tool_path(&self.tool_paths.gallery_dl);
        let (tx, rx) = mpsc::channel();
        let repaint = self.gallery_readiness.repaint.clone();
        #[cfg(any(test, cache_review_gui))]
        let events = self.gallery_readiness.events.clone();
        self.gallery_readiness = GalleryReadinessState {
            configured_path: self.tool_paths.gallery_dl.clone(),
            resolved_path: path.clone(),
            result: None,
            pending: Some(rx),
            repaint: repaint.clone(),
            #[cfg(any(test, cache_review_gui))]
            events: events.clone(),
        };
        thread::spawn(move || {
            #[cfg(cache_review_gui)]
            crate::infrastructure::cache_review_gui::wait_for_readiness_fixture();
            let result = crate::infrastructure::gallery_runtime_policy::readiness(&path);
            let delivered = tx.send(result).is_ok();
            #[cfg(any(test, cache_review_gui))]
            gallery_review_event(&events, "completed", &path);
            if delivered && let Some(ctx) = repaint.get() {
                #[cfg(any(test, cache_review_gui))]
                gallery_review_event(&events, "wake-requested", &path);
                ctx.request_repaint();
            }
        });
    }

    pub(super) fn poll_gallery_readiness(&mut self) {
        if self.gallery_readiness.configured_path != self.tool_paths.gallery_dl {
            self.request_gallery_readiness(true);
        }
        let result = self
            .gallery_readiness
            .pending
            .as_ref()
            .and_then(|rx| match rx.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(
                    "Gallery readiness worker ended without a result.".into(),
                )),
            });
        if let Some(result) = result {
            if self.last_action == GALLERY_CHECKING_DETAIL {
                self.last_action.clear();
            }
            #[cfg(any(test, cache_review_gui))]
            gallery_review_event(
                &self.gallery_readiness.events,
                "consumed",
                &self.gallery_readiness.resolved_path,
            );
            self.gallery_readiness.pending = None;
            self.gallery_readiness.result = Some(result);
            // The report consumes this same result, never a second probe.
            let refreshed = collect_dependency_presence_report(
                &self.tool_paths,
                self.gallery_readiness.result.as_ref(),
            );
            if let Some(gallery) = refreshed
                .requirements
                .into_iter()
                .find(|r| r.id == "tool-gallery-dl")
            {
                if let Some(current) = self
                    .prepare_report
                    .requirements
                    .iter_mut()
                    .find(|r| r.id == gallery.id)
                {
                    *current = gallery;
                } else {
                    self.prepare_report.requirements.push(gallery);
                }
            }
        }
    }

    pub fn gallery_is_ready_for_display(&self) -> bool {
        self.gallery_readiness.configured_path == self.tool_paths.gallery_dl
            && matches!(self.gallery_readiness.result, Some(Ok(())))
    }

    pub fn gallery_readiness_detail(&self) -> String {
        let detail = match &self.gallery_readiness.result {
            None => GALLERY_CHECKING_DETAIL.into(),
            Some(Ok(())) => format!(
                "Compatible gallery-dl runtime: {}",
                self.gallery_readiness.resolved_path.display()
            ),
            Some(Err(error)) => error.clone(),
        };
        detail
    }

    pub fn gallery_readiness_pending(&self) -> bool {
        self.gallery_readiness.configured_path != self.tool_paths.gallery_dl
            || self.gallery_readiness.result.is_none()
    }

    #[cfg(any(test, cache_review_gui))]
    pub(crate) fn review_gallery_events(&self) -> Vec<serde_json::Value> {
        self.gallery_readiness.events.lock().unwrap().clone()
    }
}

#[cfg(any(test, cache_review_gui))]
fn gallery_review_event(events: &Mutex<Vec<serde_json::Value>>, stage: &str, path: &Path) {
    events.lock().unwrap().push(serde_json::json!({
        "stage": stage, "path": path, "thread": format!("{:?}", thread::current().id()),
        "unix_ms": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
    }));
}
