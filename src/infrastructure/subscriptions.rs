//! Channel and playlist subscriptions in `<application-root>/data/subscriptions.yaml`.
//!
//! A check lists the newest entries with `yt-dlp --flat-playlist` and reports
//! the ones not seen before. The first check of a subscription only records
//! what already exists, so subscribing never floods the queue with old items.
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ToolPaths;

/// Newest playlist entries inspected per check.
pub const SUBSCRIPTION_SCAN_LIMIT: usize = 30;
/// Seen IDs kept per subscription; older ones cannot reappear in the scan window.
const SEEN_ID_LIMIT: usize = 500;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Subscription {
    pub url: String,
    pub name: String,
    pub enabled: bool,
    /// True after the first successful check recorded the existing entries.
    pub initialized: bool,
    pub seen_ids: Vec<String>,
    pub last_checked: String,
    pub last_checked_unix: u64,
    pub last_error: String,
    pub new_items_total: u64,
}

impl Subscription {
    pub fn new(url: &str) -> Option<Self> {
        let url = normalize_subscription_url(url);
        if super::url_host(&url).is_none() {
            return None;
        }
        Some(Self {
            url,
            name: String::new(),
            enabled: true,
            ..Default::default()
        })
    }

    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() {
            &self.url
        } else {
            &self.name
        }
    }

    /// Merge a successful scan. Returns new entries, oldest first.
    pub fn apply_scan(
        &mut self,
        scan: &SubscriptionScan,
        checked_at: &str,
        unix: u64,
    ) -> Vec<SubscriptionEntry> {
        self.last_checked = checked_at.to_owned();
        self.last_checked_unix = unix;
        self.last_error.clear();
        if self.name.trim().is_empty() && !scan.title.trim().is_empty() {
            self.name = scan.title.trim().to_owned();
        }
        let seen = self.seen_ids.iter().cloned().collect::<HashSet<_>>();
        let mut fresh = scan
            .entries
            .iter()
            .filter(|entry| !seen.contains(&entry.id))
            .cloned()
            .collect::<Vec<_>>();
        let first_check = !self.initialized;
        self.initialized = true;
        for entry in &scan.entries {
            if !seen.contains(&entry.id) && !self.seen_ids.contains(&entry.id) {
                self.seen_ids.push(entry.id.clone());
            }
        }
        if self.seen_ids.len() > SEEN_ID_LIMIT {
            let excess = self.seen_ids.len() - SEEN_ID_LIMIT;
            self.seen_ids.drain(..excess);
        }
        if first_check {
            return Vec::new();
        }
        // Channel and playlist listings put the newest entry first.
        fresh.reverse();
        self.new_items_total += fresh.len() as u64;
        fresh
    }

    pub fn record_error(&mut self, error: &str, checked_at: &str, unix: u64) {
        self.last_checked = checked_at.to_owned();
        self.last_checked_unix = unix;
        self.last_error = error.lines().last().unwrap_or(error).trim().to_owned();
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SubscriptionStore {
    pub subscriptions: Vec<Subscription>,
}

impl SubscriptionStore {
    pub fn path() -> PathBuf {
        super::runtime_paths::application_root()
            .join("data")
            .join("subscriptions.yaml")
    }

    pub fn load() -> Self {
        let mut store: Self = super::yaml_store::read_yaml_file(&Self::path()).unwrap_or_default();
        store
            .subscriptions
            .retain(|subscription| super::url_host(&subscription.url).is_some());
        store
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
        }
        let text = serde_yaml::to_string(self)
            .map_err(|error| format!("Could not serialize subscriptions: {error}"))?;
        super::yaml_store::atomic_write(&path, text.as_bytes())
    }

    /// Add a subscription; `false` for an invalid or duplicate URL.
    pub fn add(&mut self, url: &str) -> bool {
        let Some(subscription) = Subscription::new(url) else {
            return false;
        };
        if self
            .subscriptions
            .iter()
            .any(|existing| existing.url.eq_ignore_ascii_case(&subscription.url))
        {
            return false;
        }
        self.subscriptions.push(subscription);
        true
    }
}

/// A YouTube channel root lists its tabs (Videos, Shorts, Live) instead of
/// videos when flattened, so subscribe to its Videos tab.
pub fn normalize_subscription_url(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    let Some(host) = super::url_host(trimmed) else {
        return trimmed.to_owned();
    };
    if host != "youtube.com" && !host.ends_with(".youtube.com") {
        return trimmed.to_owned();
    }
    let after_scheme = trimmed.split_once("://").map_or(trimmed, |(_, rest)| rest);
    let Some((_, path)) = after_scheme.split_once('/') else {
        return trimmed.to_owned();
    };
    if path.contains(['?', '#']) {
        return trimmed.to_owned();
    }
    let segments = path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let channel_root = match segments.as_slice() {
        [handle] => handle.starts_with('@'),
        [kind, _] => matches!(*kind, "channel" | "c" | "user"),
        _ => false,
    };
    if channel_root {
        format!("{trimmed}/videos")
    } else {
        trimmed.to_owned()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionEntry {
    pub id: String,
    pub url: String,
    pub title: String,
}

#[derive(Clone, Debug, Default)]
pub struct SubscriptionScan {
    pub title: String,
    pub entries: Vec<SubscriptionEntry>,
}

/// Parse `--flat-playlist --dump-json` lines into playlist entries.
pub fn parse_flat_playlist_lines<'a>(lines: impl IntoIterator<Item = &'a str>) -> SubscriptionScan {
    let mut scan = SubscriptionScan::default();
    let mut ids = HashSet::new();
    for line in lines {
        let Ok(entry) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        if scan.title.is_empty() {
            if let Some(title) = entry
                .get("playlist_title")
                .or_else(|| entry.get("playlist_uploader"))
                .or_else(|| entry.get("channel"))
                .and_then(Value::as_str)
            {
                scan.title = title.trim().to_owned();
            }
        }
        let Some(url) = super::playlist_entry_url(&entry).or_else(|| {
            entry
                .get("url")
                .and_then(Value::as_str)
                .filter(|url| url.starts_with("http"))
                .map(str::to_owned)
        }) else {
            continue;
        };
        let id = entry
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| url.clone());
        if !ids.insert(id.clone()) {
            continue;
        }
        let title = entry
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        scan.entries.push(SubscriptionEntry { id, url, title });
    }
    scan
}

/// List the newest entries of one subscription. Runs yt-dlp and blocks.
pub fn scan_subscription(
    tool_paths: &ToolPaths,
    url: &str,
    use_cookies: bool,
    cancel: &AtomicBool,
) -> Result<SubscriptionScan, String> {
    let _storage_use = super::storage_paths::use_cache()?;
    let mut command = tool_paths.prepare_playlist_discovery_command(
        url,
        use_cookies,
        Some(SUBSCRIPTION_SCAN_LIMIT),
    )?;
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Could not start yt-dlp: {error}"))?;
    let _guard = super::track_child_process(&child, "yt-dlp subscription check");
    let stderr = child.stderr.take().map(|mut stderr| {
        thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        })
    });
    let mut lines = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines() {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill();
                break;
            }
            match line {
                Ok(line) if !line.trim().is_empty() => lines.push(line),
                Ok(_) => {}
                Err(_) => break,
            }
        }
    }
    let status = child.wait().map_err(|error| error.to_string())?;
    let stderr = stderr
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();
    if cancel.load(Ordering::Relaxed) {
        return Err("Subscription check cancelled.".to_owned());
    }
    let scan = parse_flat_playlist_lines(lines.iter().map(String::as_str));
    if scan.entries.is_empty() {
        let detail = stderr
            .lines()
            .rev()
            .find(|line| line.contains("ERROR"))
            .unwrap_or_else(|| stderr.lines().last().unwrap_or_default())
            .trim()
            .to_owned();
        return Err(if detail.is_empty() {
            format!("No entries found (yt-dlp exit status {status}).")
        } else {
            detail
        });
    }
    Ok(scan)
}

pub struct SubscriptionCheckResult {
    pub url: String,
    pub result: Result<SubscriptionScan, String>,
}

/// Check the given subscription URLs one after another on a worker thread.
pub fn spawn_subscription_check(
    tool_paths: ToolPaths,
    urls: Vec<String>,
    use_cookies: bool,
    cancel: Arc<AtomicBool>,
    tx: std::sync::mpsc::Sender<SubscriptionCheckResult>,
) {
    thread::spawn(move || {
        for url in urls {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let result = scan_subscription(&tool_paths, &url, use_cookies, &cancel);
            if tx.send(SubscriptionCheckResult { url, result }).is_err() {
                break;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(id: &str, title: &str) -> String {
        format!(
            r#"{{"id":"{id}","title":"{title}","webpage_url":"https://www.youtube.com/watch?v={id}","playlist_title":"Channel - Videos"}}"#
        )
    }

    fn scan(ids: &[&str]) -> SubscriptionScan {
        let lines = ids.iter().map(|id| line(id, id)).collect::<Vec<_>>();
        parse_flat_playlist_lines(lines.iter().map(String::as_str))
    }

    #[test]
    fn parses_entries_and_title() {
        let mut lines = vec![
            line("a", "First"),
            "not json".to_owned(),
            line("b", "Second"),
        ];
        lines.push(line("a", "Duplicate"));
        let scan = parse_flat_playlist_lines(lines.iter().map(String::as_str));
        assert_eq!(scan.title, "Channel - Videos");
        assert_eq!(scan.entries.len(), 2);
        assert_eq!(scan.entries[0].url, "https://www.youtube.com/watch?v=a");
        assert_eq!(scan.entries[1].title, "Second");
    }

    #[test]
    fn first_check_records_without_reporting() {
        let mut subscription =
            Subscription::new("https://www.youtube.com/@channel/videos").unwrap();
        assert!(
            subscription
                .apply_scan(&scan(&["c", "b", "a"]), "now", 1)
                .is_empty()
        );
        assert!(subscription.initialized);
        assert_eq!(subscription.name, "Channel - Videos");
        let fresh = subscription.apply_scan(&scan(&["e", "d", "c", "b"]), "later", 2);
        assert_eq!(
            fresh
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["d", "e"],
            "new entries are reported oldest first"
        );
        assert!(
            subscription
                .apply_scan(&scan(&["e", "d"]), "again", 3)
                .is_empty()
        );
        assert_eq!(subscription.new_items_total, 2);
    }

    #[test]
    fn store_rejects_invalid_and_duplicate_urls() {
        let mut store = SubscriptionStore::default();
        assert!(store.add("https://example.com/list"));
        assert!(!store.add("HTTPS://EXAMPLE.COM/list"));
        assert!(!store.add("not a url"));
        assert_eq!(store.subscriptions.len(), 1);
    }

    #[test]
    fn youtube_channel_roots_use_the_videos_tab() {
        assert_eq!(
            normalize_subscription_url(" https://www.youtube.com/@name/ "),
            "https://www.youtube.com/@name/videos"
        );
        assert_eq!(
            normalize_subscription_url("https://youtube.com/channel/UC123"),
            "https://youtube.com/channel/UC123/videos"
        );
        for unchanged in [
            "https://www.youtube.com/@name/streams",
            "https://www.youtube.com/playlist?list=PL1",
            "https://www.bilibili.com/@name",
            "https://www.youtube.com",
        ] {
            assert_eq!(normalize_subscription_url(unchanged), unchanged);
        }
    }

    #[test]
    fn errors_keep_the_last_line() {
        let mut subscription = Subscription::new("https://example.com/a").unwrap();
        subscription.record_error("warning\nERROR: gone", "now", 5);
        assert_eq!(subscription.last_error, "ERROR: gone");
        assert_eq!(subscription.last_checked_unix, 5);
    }
}
