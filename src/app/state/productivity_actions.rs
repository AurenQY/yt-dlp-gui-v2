//! Queue productivity features: parallel-slot dispatch helpers, retrying
//! failed items, URL import, queue filtering, download history, subscriptions,
//! per-site folders, scheduled limits/start and the queue-finished action.
use super::*;
use crate::infrastructure::download_history::{DownloadHistory, HistoryEntry};
use crate::infrastructure::local_time;
use crate::infrastructure::subscriptions::{
    Subscription, SubscriptionCheckResult, SubscriptionStore, spawn_subscription_check,
};

/// Countdown before the queue-finished sleep/shutdown action runs.
pub const AFTER_QUEUE_COUNTDOWN: Duration = Duration::from_secs(30);
/// Automatic subscription checks wait this long after launch.
const SUBSCRIPTION_STARTUP_DELAY: Duration = Duration::from_secs(60);
/// How often due subscriptions are evaluated while the app runs.
const SUBSCRIPTION_DUE_POLL: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QueueStatusFilter {
    #[default]
    All,
    Pending,
    Completed,
    Failed,
}

impl QueueStatusFilter {
    pub const ALL: [Self; 4] = [Self::All, Self::Pending, Self::Completed, Self::Failed];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::All => "item.all",
            Self::Pending => "item.queued",
            Self::Completed => "item.done",
            Self::Failed => "item.failed",
        }
    }
}

/// One-shot action after every queued download has finished. Session only:
/// it is never saved, so a restart cannot unexpectedly shut the PC down.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AfterQueueAction {
    #[default]
    Nothing,
    OpenFolder,
    Sleep,
    Shutdown,
}

impl AfterQueueAction {
    pub const ALL: [Self; 4] = [Self::Nothing, Self::OpenFolder, Self::Sleep, Self::Shutdown];

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Nothing => "queue.after.nothing",
            Self::OpenFolder => "queue.after.open_folder",
            Self::Sleep => "queue.after.sleep",
            Self::Shutdown => "queue.after.shutdown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TaskbarProgress {
    Hidden,
    Indeterminate,
    Normal(f32),
    Error(f32),
}

pub(crate) struct ProductivityState {
    pending_imports: VecDeque<String>,
    last_injected_import: Option<String>,
    pub(super) queue_filter_text: String,
    pub(super) queue_filter: QueueStatusFilter,
    after_queue_action: AfterQueueAction,
    after_queue_deadline: Option<Instant>,
    queue_run_active: bool,
    run_baseline_done: usize,
    run_baseline_failed: usize,
    scheduled_start_minute: Option<u32>,
    scheduled_start_deadline: Option<Instant>,
    history: DownloadHistory,
    pub(super) history_search: String,
    subscriptions: SubscriptionStore,
    subscription_rx: Option<Receiver<SubscriptionCheckResult>>,
    subscription_cancel: Option<Arc<AtomicBool>>,
    subscription_checks_remaining: usize,
    subscription_new_found: usize,
    started_at: Instant,
    last_subscription_due_poll: Option<Instant>,
    pub(super) subscription_url_draft: String,
    pub(super) site_rule_domain_draft: String,
    pub(super) site_rule_dir_draft: String,
    pub(super) scheduled_start_draft: String,
}

impl ProductivityState {
    pub(super) fn load() -> Self {
        Self {
            pending_imports: VecDeque::new(),
            last_injected_import: None,
            queue_filter_text: String::new(),
            queue_filter: QueueStatusFilter::All,
            after_queue_action: AfterQueueAction::Nothing,
            after_queue_deadline: None,
            queue_run_active: false,
            run_baseline_done: 0,
            run_baseline_failed: 0,
            scheduled_start_minute: None,
            scheduled_start_deadline: None,
            history: DownloadHistory::load(),
            history_search: String::new(),
            subscriptions: SubscriptionStore::load(),
            subscription_rx: None,
            subscription_cancel: None,
            subscription_checks_remaining: 0,
            subscription_new_found: 0,
            started_at: Instant::now(),
            last_subscription_due_poll: None,
            subscription_url_draft: String::new(),
            site_rule_domain_draft: String::new(),
            site_rule_dir_draft: String::new(),
            scheduled_start_draft: "02:00".to_owned(),
        }
    }
}

impl Drop for ProductivityState {
    fn drop(&mut self) {
        if let Some(cancel) = self.subscription_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn item_finished_like(item: &QueueItem) -> bool {
    matches!(
        item_latest_download_state(item),
        Some(WorkflowState::Finished | WorkflowState::Failed | WorkflowState::Cancelled)
    )
}

impl AppState {
    /// Called once per frame from the app loop.
    pub fn poll_productivity(&mut self) {
        self.drain_pending_imports();
        self.poll_scheduled_queue_start();
        self.observe_queue_run();
        self.poll_after_queue_countdown();
        self.poll_subscription_results();
        self.maybe_start_due_subscription_checks();
    }

    /// Productivity timers that must tick while the window is otherwise idle.
    pub fn productivity_repaint_interval(&self) -> Option<Duration> {
        if self.productivity.after_queue_deadline.is_some()
            || self.productivity.scheduled_start_deadline.is_some()
            || !self.productivity.pending_imports.is_empty()
            || self.productivity.subscription_rx.is_some()
        {
            return Some(Duration::from_millis(500));
        }
        if self.config.subscriptions_auto_check
            && self
                .productivity
                .subscriptions
                .subscriptions
                .iter()
                .any(|subscription| subscription.enabled)
        {
            return Some(SUBSCRIPTION_DUE_POLL);
        }
        None
    }

    // ---- dispatch helpers -------------------------------------------------

    /// Tool settings for a new download: the scheduled speed limit replaces
    /// the regular one while its daily window is active.
    pub(super) fn download_dispatch_tool_paths(&self) -> ToolPaths {
        let mut paths = self.tool_paths.clone();
        if let Some(rate) = self.active_scheduled_rate_limit() {
            paths.limit_rate = rate;
        }
        paths
    }

    pub fn active_scheduled_rate_limit(&self) -> Option<String> {
        self.config
            .limit_rate_schedule
            .active_rate(local_time::now().minute_of_day())
            .map(str::to_owned)
    }

    /// The first matching site rule replaces the default download folder.
    /// An item-specific folder and a yt-dlp config that owns output win.
    pub(super) fn effective_item_output_dir(&self, item: &QueueItem) -> String {
        if item.selection.output_dir == self.item_defaults.output_dir
            && !self.config.site_rules.is_empty()
            && !self.output_dir_locked_by_config()
        {
            if let Some(rule) = self
                .config
                .site_rules
                .iter()
                .find(|rule| rule.matches_url(&item.source_url))
            {
                return rule.output_dir.clone();
            }
        }
        item.selection.output_dir.clone()
    }

    // ---- retry failed -----------------------------------------------------

    pub fn failed_queue_item_count(&self) -> usize {
        self.queue_items
            .iter()
            .filter(|item| matches!(item_summary_bucket(item), QueueSummaryBucket::Failed))
            .count()
    }

    pub fn can_retry_failed_queue_items(&self) -> bool {
        self.app_mode != AppMode::Origin
            && !self.queue_display_mode_is_audio()
            && self.failed_queue_item_count() > 0
    }

    /// Re-queue every failed item and start as many as the slots allow.
    pub fn retry_failed_queue_items(&mut self) {
        if !self.can_retry_failed_queue_items() {
            return;
        }
        if let Err(error) = self.ensure_yt_dlp_ready() {
            self.last_action = error;
            return;
        }
        let ids = self
            .queue_items
            .iter()
            .enumerate()
            .filter(|(index, item)| {
                matches!(item_summary_bucket(item), QueueSummaryBucket::Failed)
                    && item_can_enter_download_queue(item)
                    && !self.item_is_busy(*index)
            })
            .map(|(_, item)| item.id)
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return;
        }
        for item_id in &ids {
            self.enqueue_download_ready_item(*item_id);
        }
        self.fill_download_slots(None);
        self.last_action = i18n::format_fixed_english(
            "Retrying {count} failed items.",
            &[("{count}", &ids.len().to_string())],
        );
    }

    // ---- URL import -------------------------------------------------------

    pub fn pending_import_count(&self) -> usize {
        self.productivity.pending_imports.len()
    }

    /// Queue URLs to be added one at a time through the regular add flow.
    pub fn enqueue_import_urls(&mut self, urls: Vec<String>) -> usize {
        let existing = self
            .queue_items
            .iter()
            .map(|item| canonical_queue_source_key(&item.source_url))
            .chain(
                self.productivity
                    .pending_imports
                    .iter()
                    .map(|url| canonical_queue_source_key(url)),
            )
            .collect::<std::collections::HashSet<_>>();
        let mut added = 0usize;
        let mut seen = std::collections::HashSet::new();
        for url in urls {
            let key = canonical_queue_source_key(&url);
            if existing.contains(&key) || !seen.insert(key) {
                continue;
            }
            self.productivity.pending_imports.push_back(url);
            added += 1;
        }
        self.last_action = if added == 0 {
            "No new URLs to import.".to_owned()
        } else if self.app_mode == AppMode::Origin {
            i18n::format_fixed_english(
                "{count} URLs are waiting. Switch to Standard Mode to add them to the queue.",
                &[("{count}", &added.to_string())],
            )
        } else {
            i18n::format_fixed_english(
                "Importing {count} URLs.",
                &[("{count}", &added.to_string())],
            )
        };
        added
    }

    pub fn import_urls_from_text(&mut self, text: &str) -> usize {
        let urls = crate::infrastructure::url_import::extract_urls(text);
        if urls.is_empty() {
            self.last_action = "No URLs found to import.".to_owned();
            return 0;
        }
        self.enqueue_import_urls(urls)
    }

    pub fn import_urls_from_files(&mut self, paths: &[PathBuf]) -> usize {
        let mut urls = Vec::new();
        let mut errors = Vec::new();
        for path in paths {
            match crate::infrastructure::url_import::urls_from_file(path) {
                Ok(found) => urls.extend(found),
                Err(error) => errors.push(error),
            }
        }
        if urls.is_empty() {
            self.last_action = errors
                .first()
                .cloned()
                .unwrap_or_else(|| "No URLs found to import.".to_owned());
            return 0;
        }
        let added = self.enqueue_import_urls(urls);
        for error in errors {
            self.push_runtime_log(format!("Import skipped: {error}"));
        }
        added
    }

    /// Paste outside a text field: one URL uses the normal add action,
    /// several URLs are imported.
    pub fn paste_urls_from_text(&mut self, text: &str) {
        let urls = crate::infrastructure::url_import::extract_urls(text);
        match urls.len() {
            0 => {}
            1 if !self.url_input_locked() => {
                self.url_input = urls[0].clone();
                self.run_primary_url_action();
            }
            _ => {
                self.enqueue_import_urls(urls);
            }
        }
    }

    pub fn cancel_pending_imports(&mut self) {
        let count = self.productivity.pending_imports.len();
        self.productivity.pending_imports.clear();
        if count > 0 {
            self.last_action = i18n::format_fixed_english(
                "Cancelled {count} pending imports.",
                &[("{count}", &count.to_string())],
            );
        }
    }

    fn drain_pending_imports(&mut self) {
        if self.app_mode == AppMode::Origin
            || self.url_input_locked()
            || self.format_picker.open
            || self.gallery_range_picker.item_id.is_some()
        {
            return;
        }
        let current = self.url_input.trim().to_owned();
        let owned_by_import =
            self.productivity.last_injected_import.as_deref() == Some(current.as_str());
        if !current.is_empty() && !owned_by_import {
            // The user is typing; never overwrite their input.
            return;
        }
        let Some(url) = self.productivity.pending_imports.pop_front() else {
            if owned_by_import {
                self.url_input.clear();
                self.productivity.last_injected_import = None;
            }
            return;
        };
        if let Err(error) = self.ensure_yt_dlp_ready() {
            self.productivity.pending_imports.push_front(url);
            self.last_action = error;
            return;
        }
        self.url_input = url.clone();
        self.productivity.last_injected_import = Some(url);
        if self.queue_display_mode_is_audio() {
            self.add_current_url_to_music_compact_batch();
        } else {
            self.add_current_url_to_batch();
        }
    }

    // ---- queue filter -----------------------------------------------------

    pub fn queue_filter(&self) -> QueueStatusFilter {
        self.productivity.queue_filter
    }

    pub fn set_queue_filter(&mut self, filter: QueueStatusFilter) {
        self.productivity.queue_filter = filter;
    }

    pub fn queue_filter_text_mut(&mut self) -> &mut String {
        &mut self.productivity.queue_filter_text
    }

    pub fn queue_filter_is_active(&self) -> bool {
        self.productivity.queue_filter != QueueStatusFilter::All
            || !self.productivity.queue_filter_text.trim().is_empty()
    }

    pub fn clear_queue_filter(&mut self) {
        self.productivity.queue_filter = QueueStatusFilter::All;
        self.productivity.queue_filter_text.clear();
    }

    pub fn queue_item_matches_filter(&self, index: usize) -> bool {
        let Some(item) = self.queue_items.get(index) else {
            return false;
        };
        let status_matches = match self.productivity.queue_filter {
            QueueStatusFilter::All => true,
            QueueStatusFilter::Pending => {
                matches!(item_summary_bucket(item), QueueSummaryBucket::Queued)
            }
            QueueStatusFilter::Completed => {
                matches!(item_summary_bucket(item), QueueSummaryBucket::Completed)
            }
            QueueStatusFilter::Failed => {
                matches!(item_summary_bucket(item), QueueSummaryBucket::Failed)
            }
        };
        if !status_matches {
            return false;
        }
        let query = self.productivity.queue_filter_text.trim();
        if query.is_empty() {
            return true;
        }
        let uploader = item
            .metadata()
            .map(|metadata| metadata.uploader.as_str())
            .unwrap_or_default();
        let haystack = format!(
            "{}\n{}\n{}\n{}",
            item.title, item.source_url, uploader, item.selection.file_name
        )
        .to_lowercase();
        query
            .split_whitespace()
            .all(|term| haystack.contains(&term.to_lowercase()))
    }

    // ---- download history ------------------------------------------------

    pub(super) fn record_download_history(&mut self, item_id: QueueItemId, output_path: &str) {
        if !self.config.history_enabled {
            return;
        }
        let Some(item) = self.queue_item_by_id(item_id) else {
            return;
        };
        let title = if item.title.trim().is_empty() {
            Path::new(output_path)
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default()
        } else {
            item.title.clone()
        };
        let entry = HistoryEntry {
            source_key: canonical_queue_source_key(&item.source_url),
            url: item.source_url.clone(),
            title,
            output_path: output_path.to_owned(),
            downloaded_at: local_time::now().display(),
            timestamp: unix_now(),
        };
        self.productivity.history.record(entry);
        if let Err(error) = self.productivity.history.save() {
            self.push_runtime_log(format!("Could not save download history: {error}"));
        }
    }

    /// "Downloaded before" hint for an item that has not completed this session.
    pub fn item_history_hint(&self, index: usize) -> Option<String> {
        if !self.config.history_enabled {
            return None;
        }
        let item = self.queue_items.get(index)?;
        if item.workflows.iter().any(|run| {
            matches!(
                run.kind,
                WorkflowKind::DownloadMedia | WorkflowKind::ExportMedia | WorkflowKind::PostProcess
            ) && matches!(
                run.state,
                WorkflowState::Queued | WorkflowState::Running | WorkflowState::Finished
            )
        }) {
            return None;
        }
        let entry = self
            .productivity
            .history
            .latest_for_source(&canonical_queue_source_key(&item.source_url))?;
        Some(self.ui_i18n_text_with_replacements(
            "item.downloaded_before",
            &[("{date}", entry.downloaded_at.as_str())],
        ))
    }

    pub fn history(&self) -> &DownloadHistory {
        &self.productivity.history
    }

    pub fn history_search_mut(&mut self) -> &mut String {
        &mut self.productivity.history_search
    }

    pub fn history_search_results(&self) -> Vec<usize> {
        self.productivity
            .history
            .search(&self.productivity.history_search)
    }

    pub fn remove_history_entry(&mut self, index: usize) {
        if self.productivity.history.remove(index).is_some() {
            if let Err(error) = self.productivity.history.save() {
                self.last_action = error;
            }
        }
    }

    pub fn clear_history(&mut self) {
        self.productivity.history.clear();
        if let Err(error) = self.productivity.history.save() {
            self.last_action = error;
        }
    }

    pub fn requeue_history_entry(&mut self, index: usize) {
        let Some(url) = self
            .productivity
            .history
            .entries
            .get(index)
            .map(|entry| entry.url.clone())
        else {
            return;
        };
        self.enqueue_import_urls(vec![url]);
    }

    pub fn set_history_enabled(&mut self, enabled: bool) {
        self.config.history_enabled = enabled;
        let _ = self.config.save();
    }

    // ---- queue run / taskbar / after-queue action -------------------------

    fn observe_queue_run(&mut self) {
        let busy = self.has_running_download_workflow();
        if busy && !self.productivity.queue_run_active {
            self.productivity.queue_run_active = true;
            self.productivity.run_baseline_done = self
                .queue_items
                .iter()
                .filter(|item| item_finished_like(item))
                .count();
            self.productivity.run_baseline_failed = self.failed_queue_item_count();
            return;
        }
        if busy || !self.productivity.queue_run_active {
            return;
        }
        let waiting = self
            .queue_items
            .iter()
            .any(|item| item_latest_download_state(item).is_some_and(is_pending_download_state));
        if waiting {
            return;
        }
        self.productivity.queue_run_active = false;
        self.on_queue_run_finished();
    }

    pub fn taskbar_progress(&self) -> TaskbarProgress {
        if !self.has_running_download_workflow() {
            return TaskbarProgress::Hidden;
        }
        let mut running = 0usize;
        let mut running_progress = 0.0f32;
        let mut queued = 0usize;
        let mut done = 0usize;
        for item in &self.queue_items {
            let active = item
                .workflows
                .iter()
                .filter(|run| {
                    matches!(
                        run.kind,
                        WorkflowKind::DownloadMedia
                            | WorkflowKind::ExportMedia
                            | WorkflowKind::PostProcess
                    ) && run.state == WorkflowState::Running
                })
                .map(|run| run.progress)
                .fold(None, |best: Option<f32>, value| {
                    Some(best.map_or(value, |b| b.max(value)))
                });
            if let Some(progress) = active {
                running += 1;
                running_progress += progress.clamp(0.0, 100.0) / 100.0;
            } else if item_latest_download_state(item).is_some_and(is_pending_download_state) {
                queued += 1;
            } else if item_finished_like(item) {
                done += 1;
            }
        }
        let done = done.saturating_sub(self.productivity.run_baseline_done);
        let total = done + running + queued;
        if total == 0 {
            return TaskbarProgress::Indeterminate;
        }
        let fraction = ((done as f32 + running_progress) / total as f32).clamp(0.0, 1.0);
        if fraction <= f32::EPSILON {
            TaskbarProgress::Indeterminate
        } else if self.failed_queue_item_count() > self.productivity.run_baseline_failed {
            TaskbarProgress::Error(fraction)
        } else {
            TaskbarProgress::Normal(fraction)
        }
    }

    pub fn after_queue_action(&self) -> AfterQueueAction {
        self.productivity.after_queue_action
    }

    pub fn set_after_queue_action(&mut self, action: AfterQueueAction) {
        self.productivity.after_queue_action = action;
        self.productivity.after_queue_deadline = None;
    }

    pub fn after_queue_countdown_remaining(&self) -> Option<u64> {
        self.productivity
            .after_queue_deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()).as_secs() + 1)
    }

    pub fn cancel_after_queue_action(&mut self) {
        self.productivity.after_queue_deadline = None;
        self.productivity.after_queue_action = AfterQueueAction::Nothing;
        self.last_action = "Queue finish action cancelled.".to_owned();
    }

    fn on_queue_run_finished(&mut self) {
        match self.productivity.after_queue_action {
            AfterQueueAction::Nothing => {}
            AfterQueueAction::OpenFolder => {
                self.productivity.after_queue_action = AfterQueueAction::Nothing;
                let folder = resolve_output_dir(&self.item_defaults.output_dir)
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|_| self.item_defaults.output_dir.clone());
                if let Err(error) = crate::infrastructure::open_output_folder(&folder) {
                    self.last_action = error;
                }
            }
            AfterQueueAction::Sleep | AfterQueueAction::Shutdown => {
                self.productivity.after_queue_deadline =
                    Some(Instant::now() + AFTER_QUEUE_COUNTDOWN);
            }
        }
    }

    fn poll_after_queue_countdown(&mut self) {
        let Some(deadline) = self.productivity.after_queue_deadline else {
            return;
        };
        if self.has_running_download_workflow() {
            // New work started during the countdown: wait for it instead.
            self.productivity.after_queue_deadline = None;
            self.productivity.queue_run_active = true;
            return;
        }
        if Instant::now() < deadline {
            return;
        }
        self.productivity.after_queue_deadline = None;
        let action = std::mem::take(&mut self.productivity.after_queue_action);
        let result = match action {
            AfterQueueAction::Sleep => crate::infrastructure::system_power::sleep_computer(),
            AfterQueueAction::Shutdown => {
                self.flush_queue_persistence_before_exit();
                crate::infrastructure::system_power::shutdown_computer()
            }
            AfterQueueAction::Nothing | AfterQueueAction::OpenFolder => Ok(()),
        };
        if let Err(error) = result {
            self.last_action = error;
        }
    }

    // ---- scheduled queue start ---------------------------------------------

    pub fn scheduled_queue_start(&self) -> Option<String> {
        self.productivity
            .scheduled_start_minute
            .map(local_time::format_hhmm)
    }

    pub fn scheduled_start_draft_mut(&mut self) -> &mut String {
        &mut self.productivity.scheduled_start_draft
    }

    /// Start the download queue at the next local `HH:MM`.
    pub fn schedule_queue_start_from_draft(&mut self) -> bool {
        let Some(minute) = local_time::parse_hhmm(&self.productivity.scheduled_start_draft) else {
            self.last_action = "Enter a start time as HH:MM.".to_owned();
            return false;
        };
        let delay = local_time::seconds_until(local_time::now(), minute);
        self.productivity.scheduled_start_minute = Some(minute);
        self.productivity.scheduled_start_deadline =
            Some(Instant::now() + Duration::from_secs(delay));
        self.productivity.scheduled_start_draft = local_time::format_hhmm(minute);
        self.last_action = i18n::format_fixed_english(
            "Downloads will start at {time}.",
            &[("{time}", &local_time::format_hhmm(minute))],
        );
        true
    }

    pub fn cancel_scheduled_queue_start(&mut self) {
        self.productivity.scheduled_start_minute = None;
        self.productivity.scheduled_start_deadline = None;
    }

    fn poll_scheduled_queue_start(&mut self) {
        let Some(deadline) = self.productivity.scheduled_start_deadline else {
            return;
        };
        if Instant::now() < deadline {
            return;
        }
        self.cancel_scheduled_queue_start();
        if self.has_running_download_workflow() {
            self.fill_download_slots(None);
        } else if self.queue_mode_downloads_as_audio() {
            match self.music.active_music_download_choice {
                Some(choice) => self.start_download_with_music_choice(choice),
                None => self.start_queue_downloads_if_ready(),
            }
        } else {
            self.start_queue_downloads_if_ready();
        }
    }

    /// Same entry as the Download button, and only when that button is
    /// enabled for starting (never toggles a running Origin download off).
    pub fn start_queue_downloads_if_ready(&mut self) {
        if self.app_mode == AppMode::Origin {
            if self.origin_download_can_start() {
                self.request_origin_download();
            }
            return;
        }
        if self.has_pending_download_items() {
            self.request_main_download();
        } else if self.queue_items.is_empty() {
            self.last_action = "There is no URL to download.".to_owned();
        }
    }

    // ---- subscriptions -------------------------------------------------------

    pub fn subscriptions(&self) -> &[Subscription] {
        &self.productivity.subscriptions.subscriptions
    }

    pub fn subscription_url_draft_mut(&mut self) -> &mut String {
        &mut self.productivity.subscription_url_draft
    }

    pub fn subscription_check_running(&self) -> bool {
        self.productivity.subscription_rx.is_some()
    }

    pub fn add_subscription_from_draft(&mut self) {
        let url = self.productivity.subscription_url_draft.trim().to_owned();
        if !self.productivity.subscriptions.add(&url) {
            self.last_action = "Enter a new channel or playlist URL.".to_owned();
            return;
        }
        self.productivity.subscription_url_draft.clear();
        self.save_subscriptions();
        // The first check only records existing entries.
        self.start_subscription_checks(vec![url]);
    }

    pub fn remove_subscription(&mut self, index: usize) {
        if index < self.productivity.subscriptions.subscriptions.len() {
            self.productivity.subscriptions.subscriptions.remove(index);
            self.save_subscriptions();
        }
    }

    pub fn set_subscription_enabled(&mut self, index: usize, enabled: bool) {
        if let Some(subscription) = self.productivity.subscriptions.subscriptions.get_mut(index) {
            subscription.enabled = enabled;
            self.save_subscriptions();
        }
    }

    pub fn check_subscriptions_now(&mut self) {
        let urls = self
            .productivity
            .subscriptions
            .subscriptions
            .iter()
            .filter(|subscription| subscription.enabled)
            .map(|subscription| subscription.url.clone())
            .collect::<Vec<_>>();
        if urls.is_empty() {
            self.last_action = "There are no enabled subscriptions.".to_owned();
            return;
        }
        self.start_subscription_checks(urls);
    }

    pub fn set_subscriptions_auto_check(&mut self, enabled: bool) {
        self.config.subscriptions_auto_check = enabled;
        let _ = self.config.save();
    }

    pub fn set_subscriptions_interval_hours(&mut self, hours: u32) {
        self.config.subscriptions_interval_hours =
            hours.clamp(1, crate::infrastructure::MAX_SUBSCRIPTION_INTERVAL_HOURS);
        let _ = self.config.save();
    }

    fn save_subscriptions(&mut self) {
        if let Err(error) = self.productivity.subscriptions.save() {
            self.last_action = error;
        }
    }

    fn start_subscription_checks(&mut self, urls: Vec<String>) {
        if self.productivity.subscription_rx.is_some() || urls.is_empty() {
            return;
        }
        if let Err(error) = self.ensure_yt_dlp_ready() {
            self.last_action = error;
            return;
        }
        let use_cookies =
            self.item_defaults.use_cookies && self.tool_paths.validate_cookie_setup(true).is_ok();
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.productivity.subscription_checks_remaining = urls.len();
        self.productivity.subscription_new_found = 0;
        self.productivity.subscription_rx = Some(rx);
        self.productivity.subscription_cancel = Some(cancel.clone());
        spawn_subscription_check(self.tool_paths.clone(), urls, use_cookies, cancel, tx);
        self.last_action = "Checking subscriptions…".to_owned();
    }

    fn poll_subscription_results(&mut self) {
        let Some(rx) = self.productivity.subscription_rx.as_ref() else {
            return;
        };
        let mut results = Vec::new();
        let mut finished = false;
        loop {
            match rx.try_recv() {
                Ok(result) => results.push(result),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    finished = true;
                    break;
                }
            }
        }
        if results.is_empty() && !finished {
            return;
        }
        let checked_at = local_time::now().display();
        let unix = unix_now();
        let mut new_urls = Vec::new();
        for result in results {
            self.productivity.subscription_checks_remaining = self
                .productivity
                .subscription_checks_remaining
                .saturating_sub(1);
            let Some(subscription) = self
                .productivity
                .subscriptions
                .subscriptions
                .iter_mut()
                .find(|subscription| subscription.url == result.url)
            else {
                continue;
            };
            match result.result {
                Ok(scan) => {
                    let fresh = subscription.apply_scan(&scan, &checked_at, unix);
                    new_urls.extend(fresh.into_iter().map(|entry| entry.url));
                }
                Err(error) => {
                    subscription.record_error(&error, &checked_at, unix);
                    let name = subscription.display_name().to_owned();
                    self.push_runtime_log(format!("Subscription check failed for {name}: {error}"));
                }
            }
        }
        self.save_subscriptions();
        if !new_urls.is_empty() {
            self.productivity.subscription_new_found += new_urls.len();
            self.enqueue_import_urls(new_urls);
        }
        if finished || self.productivity.subscription_checks_remaining == 0 {
            self.productivity.subscription_rx = None;
            self.productivity.subscription_cancel = None;
            let found = self.productivity.subscription_new_found;
            self.last_action = if found == 0 {
                "Subscriptions checked. No new items.".to_owned()
            } else {
                i18n::format_fixed_english(
                    "Subscriptions checked. {count} new items added to the import list.",
                    &[("{count}", &found.to_string())],
                )
            };
        }
    }

    fn maybe_start_due_subscription_checks(&mut self) {
        if !self.config.subscriptions_auto_check
            || self.productivity.subscription_rx.is_some()
            || self.productivity.started_at.elapsed() < SUBSCRIPTION_STARTUP_DELAY
            || self
                .productivity
                .last_subscription_due_poll
                .is_some_and(|last| last.elapsed() < SUBSCRIPTION_DUE_POLL)
        {
            return;
        }
        self.productivity.last_subscription_due_poll = Some(Instant::now());
        if self.tool_paths.validate_yt_dlp_available().is_err() {
            return;
        }
        let interval = u64::from(self.config.subscriptions_interval_hours) * 3_600;
        let now = unix_now();
        let due = self
            .productivity
            .subscriptions
            .subscriptions
            .iter()
            .filter(|subscription| {
                subscription.enabled
                    && now.saturating_sub(subscription.last_checked_unix) >= interval
            })
            .map(|subscription| subscription.url.clone())
            .collect::<Vec<_>>();
        self.start_subscription_checks(due);
    }

    // ---- settings ------------------------------------------------------------

    pub fn set_max_concurrent_downloads(&mut self, value: usize) {
        self.config.max_concurrent_downloads =
            crate::infrastructure::normalize_max_concurrent_downloads(value);
        let _ = self.config.save();
        // Use any newly allowed slot right away.
        if self.has_running_download_workflow() {
            self.fill_download_slots(None);
        }
    }

    pub fn set_auto_retry_count(&mut self, value: usize) {
        self.config.auto_retry_count = value.min(crate::infrastructure::MAX_AUTO_RETRY_COUNT);
        self.tool_paths.auto_retry_count = self.config.auto_retry_count;
        let _ = self.config.save();
    }

    pub fn set_sponsorblock_mode(&mut self, mode: crate::infrastructure::SponsorBlockMode) {
        self.config.sponsorblock_mode = mode;
        self.tool_paths.sponsorblock_mode = mode;
        let _ = self.config.save();
    }

    pub fn set_sponsorblock_categories(&mut self, value: &str) {
        // Keep the raw draft while typing; normalize when building commands.
        self.config.sponsorblock_categories = value.to_owned();
        self.tool_paths.sponsorblock_categories = value.to_owned();
        let _ = self.config.save();
    }

    pub fn set_embed_metadata(&mut self, enabled: bool) {
        self.config.embed_metadata = enabled;
        self.tool_paths.embed_metadata = enabled;
        let _ = self.config.save();
    }

    pub fn set_playlist_match_filter(&mut self, value: &str) {
        self.config.playlist_match_filter = value.to_owned();
        self.tool_paths.playlist_match_filter = value.trim().to_owned();
        let _ = self.config.save();
    }

    pub fn set_limit_rate_schedule(&mut self, schedule: crate::infrastructure::RateLimitSchedule) {
        self.config.limit_rate_schedule = schedule;
        let _ = self.config.save();
    }

    // ---- site rules --------------------------------------------------------

    pub fn site_rule_domain_draft_mut(&mut self) -> &mut String {
        &mut self.productivity.site_rule_domain_draft
    }

    pub fn site_rule_dir_draft_mut(&mut self) -> &mut String {
        &mut self.productivity.site_rule_dir_draft
    }

    pub fn add_site_rule_from_draft(&mut self) -> bool {
        let rule = crate::infrastructure::SiteRule {
            domain: self.productivity.site_rule_domain_draft.clone(),
            output_dir: self.productivity.site_rule_dir_draft.clone(),
        };
        let Some(rule) = rule.normalized() else {
            self.last_action = "Enter a site domain such as youtube.com and a folder.".to_owned();
            return false;
        };
        if let Some(existing) = self
            .config
            .site_rules
            .iter_mut()
            .find(|existing| existing.domain == rule.domain)
        {
            existing.output_dir = rule.output_dir;
        } else {
            self.config.site_rules.push(rule);
        }
        self.productivity.site_rule_domain_draft.clear();
        self.productivity.site_rule_dir_draft.clear();
        let _ = self.config.save();
        true
    }

    pub fn remove_site_rule(&mut self, index: usize) {
        if index < self.config.site_rules.len() {
            self.config.site_rules.remove(index);
            let _ = self.config.save();
        }
    }
}
