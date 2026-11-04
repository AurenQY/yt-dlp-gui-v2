//! Completed-download history in `<application-root>/data/history.yaml`.
//!
//! History is user data next to the saved queue, not a cache: cache cleanup
//! never touches it. Entries are keyed by the canonical queue source key so a
//! repeated source can be recognised before it is downloaded again.
use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Oldest entries are dropped beyond this many records.
pub const HISTORY_LIMIT: usize = 5_000;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HistoryEntry {
    pub source_key: String,
    pub url: String,
    pub title: String,
    pub output_path: String,
    /// Local `YYYY-MM-DD HH:MM` when the download completed.
    pub downloaded_at: String,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DownloadHistory {
    pub entries: Vec<HistoryEntry>,
    #[serde(skip)]
    latest_by_source: HashMap<String, usize>,
}

impl DownloadHistory {
    pub fn path() -> PathBuf {
        super::runtime_paths::application_root()
            .join("data")
            .join("history.yaml")
    }

    /// A missing or unreadable file yields an empty history; it is never
    /// overwritten until a new entry is recorded.
    pub fn load() -> Self {
        let mut history: Self =
            super::yaml_store::read_yaml_file(&Self::path()).unwrap_or_default();
        history.normalize();
        history
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
        }
        let text = serde_yaml::to_string(self)
            .map_err(|error| format!("Could not serialize download history: {error}"))?;
        super::yaml_store::atomic_write(&path, text.as_bytes())
    }

    fn normalize(&mut self) {
        self.entries
            .retain(|entry| !entry.source_key.trim().is_empty());
        if self.entries.len() > HISTORY_LIMIT {
            let excess = self.entries.len() - HISTORY_LIMIT;
            self.entries.drain(..excess);
        }
        self.rebuild_index();
    }

    fn rebuild_index(&mut self) {
        self.latest_by_source = self
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.source_key.clone(), index))
            .collect();
    }

    pub fn record(&mut self, entry: HistoryEntry) {
        if entry.source_key.trim().is_empty() {
            return;
        }
        self.entries.push(entry);
        self.normalize();
    }

    /// Most recent completed download for a source.
    pub fn latest_for_source(&self, source_key: &str) -> Option<&HistoryEntry> {
        self.latest_by_source
            .get(source_key)
            .and_then(|index| self.entries.get(*index))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn remove(&mut self, index: usize) -> Option<HistoryEntry> {
        if index >= self.entries.len() {
            return None;
        }
        let removed = self.entries.remove(index);
        self.rebuild_index();
        Some(removed)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.latest_by_source.clear();
    }

    /// Indices of entries matching every whitespace-separated term in the
    /// title, URL or output path (case-insensitive), newest first.
    pub fn search(&self, query: &str) -> Vec<usize> {
        let terms = query
            .split_whitespace()
            .map(str::to_lowercase)
            .collect::<Vec<_>>();
        self.entries
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, entry)| {
                terms.is_empty() || {
                    let haystack = format!(
                        "{}\n{}\n{}",
                        entry.title.to_lowercase(),
                        entry.url.to_lowercase(),
                        entry.output_path.to_lowercase()
                    );
                    terms.iter().all(|term| haystack.contains(term))
                }
            })
            .map(|(index, _)| index)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str, title: &str, timestamp: u64) -> HistoryEntry {
        HistoryEntry {
            source_key: key.to_owned(),
            url: format!("https://example.com/{key}"),
            title: title.to_owned(),
            output_path: format!("C:/download/{title}.mp4"),
            downloaded_at: String::new(),
            timestamp,
        }
    }

    #[test]
    fn records_latest_entry_per_source() {
        let mut history = DownloadHistory::default();
        history.record(entry("a", "First", 1));
        history.record(entry("b", "Other", 2));
        history.record(entry("a", "Again", 3));
        assert_eq!(history.len(), 3);
        assert_eq!(history.latest_for_source("a").unwrap().title, "Again");
        assert!(history.latest_for_source("missing").is_none());
        history.record(entry("", "ignored", 4));
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn search_matches_all_terms_newest_first() {
        let mut history = DownloadHistory::default();
        history.record(entry("a", "Rust Talk", 1));
        history.record(entry("b", "Cooking Show", 2));
        history.record(entry("c", "Rust Cooking", 3));
        assert_eq!(history.search("rust"), vec![2, 0]);
        assert_eq!(history.search("RUST cooking"), vec![2]);
        assert_eq!(history.search("  ").len(), 3);
    }

    #[test]
    fn enforces_limit_and_reindexes_after_removal() {
        let mut history = DownloadHistory::default();
        for index in 0..HISTORY_LIMIT + 3 {
            history.record(entry(&index.to_string(), "t", index as u64));
        }
        assert_eq!(history.len(), HISTORY_LIMIT);
        assert!(history.latest_for_source("0").is_none());
        assert!(history.latest_for_source("3").is_some());
        let removed = history.remove(0).unwrap();
        assert_eq!(removed.source_key, "3");
        assert!(history.latest_for_source("3").is_none());
        assert_eq!(
            history.latest_for_source("4").unwrap().source_key,
            "4",
            "index must follow shifted entries"
        );
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn yaml_round_trip_rebuilds_index() {
        let mut history = DownloadHistory::default();
        history.record(entry("a", "Title", 1));
        let text = serde_yaml::to_string(&history).unwrap();
        let mut decoded: DownloadHistory = serde_yaml::from_str(&text).unwrap();
        decoded.normalize();
        assert_eq!(decoded.latest_for_source("a").unwrap().title, "Title");
    }
}
