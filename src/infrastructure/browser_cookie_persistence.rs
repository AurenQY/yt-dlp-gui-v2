//! Full-site browser snapshots refresh the same managed files used by v2.
//! Runs once per Native request, before Y/G workers. Credentials stay in memory
//! and the Cookie file, never in diagnostics, the index, or analysis JSON.
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};

use super::{
    browser_native_cookies::NativeCookies,
    cookie_site_index::{
        CookieSiteIndexEntry, lock_cookie_store, read_cookie_site_index, write_cookie_site_index,
        write_cookie_store_file_atomically,
    },
    cookie_site_scope::{domain_is_in_site, site_domain_for_url},
    tool_install::portable_root_dir,
    youtube_login_rescue::site_id_from_domain,
};

pub(super) struct PersistedBrowserCookies {
    pub(super) site_id: String,
    pub(super) display_name: String,
    pub(super) cookie_count: usize,
}

/// Browser snapshots update storage only. All workflows then use the existing
/// v2 Cookie settings, with no live override or tool-specific Cookie source.
pub(super) fn prepare_shared_source_request(request: &mut Value) -> Result<(), String> {
    synchronize_before_dispatch(request)?;
    if is_cookie_workflow(request) {
        if let Some(fields) = request.as_object_mut() {
            fields.remove("browserCookies");
            fields.remove("browserCookieContext");
        }
        super::operation_diagnostics::record(
            "cookies.workflow-source",
            json!({"policy": "shared-v2-settings"}),
        );
    }
    Ok(())
}

fn is_cookie_workflow(request: &Value) -> bool {
    matches!(
        request["action"].as_str(),
        Some(
            "analyze"
                | "analyzeSelected"
                | "analyzeSources"
                | "analyzeIndexedSources"
                | "galleryRead"
                | "download"
        )
    )
}

pub(super) fn save_and_enable(request: &Value) -> Result<PersistedBrowserCookies, String> {
    let (url, root, site_id) = request_site(request)?;
    let incoming = incoming_records(request)?;
    if incoming.is_empty() {
        return Err("No browser cookies were available for this website.".to_owned());
    }
    let cookie_dir = portable_root_dir().join("data/cookies");
    let _lock = lock_cookie_store(&cookie_dir)?;
    let mut index = read_cookie_site_index(&cookie_dir)?.unwrap_or_default();
    let snapshot_ms = snapshot_timestamp(request).unwrap_or_else(now_ms);
    if index
        .sites
        .iter()
        .any(|entry| entry.id == site_id && entry.last_browser_snapshot_unix_ms > snapshot_ms)
    {
        return Err("A newer Cookie snapshot was already saved. Please save again.".to_owned());
    }
    let file_name = format!("{site_id}.cookies.txt");
    let target = cookie_dir.join(&file_name);
    let merged = merge_records(&target, &incoming, &root)?;
    let entry = if let Some(position) = index.sites.iter().position(|entry| entry.id == site_id) {
        &mut index.sites[position]
    } else {
        index.sites.push(CookieSiteIndexEntry {
            id: site_id.clone(),
            display_name: root.clone(),
            match_domains: vec![root.clone()],
            ..Default::default()
        });
        index.sites.last_mut().expect("Inserted Cookie site")
    };
    entry.login_url = url.to_owned();
    entry.cookie_file = file_name;
    entry.auto_update_from_extension = true;
    entry.last_browser_snapshot_unix_ms = snapshot_ms;
    update_record_metadata(entry, &merged);
    let display_name = entry.display_name.clone();
    // Serialize first so malformed index data cannot fail after writing Cookies.
    serde_yaml::to_string(&index).map_err(|_| "Could not serialize cookie site index.")?;
    write_records(&target, &merged)?;
    index.sites.sort_by(|left, right| left.id.cmp(&right.id));
    write_cookie_site_index(&cookie_dir, &index)?;
    record_saved(false, incoming.len());
    Ok(PersistedBrowserCookies {
        site_id,
        display_name,
        cookie_count: incoming.len(),
    })
}

pub(super) fn synchronize_before_dispatch(request: &Value) -> Result<(), String> {
    if !is_cookie_workflow(request) || !request["browserCookies"].is_array() {
        return Ok(());
    }
    // Old clients and incognito requests cannot auto-save; never infer consent.
    if request["browserCookieContext"]["incognito"].as_bool() != Some(false) {
        record_skipped("private-or-unknown-context");
        return Ok(());
    }
    let Some(snapshot_ms) = snapshot_timestamp(request) else {
        record_skipped("invalid-snapshot-time");
        return Ok(());
    };
    let (url, root, site_id) = request_site(request)?;
    let incoming = incoming_records(request)?;
    if incoming.is_empty() {
        record_skipped("empty-snapshot");
        return Ok(());
    }
    let cookie_dir = portable_root_dir().join("data/cookies");
    let _lock = lock_cookie_store(&cookie_dir)?;
    let mut index = read_cookie_site_index(&cookie_dir)?.unwrap_or_default();
    let file_name = format!("{site_id}.cookies.txt");
    let entry = if let Some(position) = index.sites.iter().position(|entry| entry.id == site_id) {
        &mut index.sites[position]
    } else {
        index.sites.push(CookieSiteIndexEntry {
            id: site_id.clone(),
            display_name: root.clone(),
            match_domains: vec![root.clone()],
            cookie_file: file_name.clone(),
            ..Default::default()
        });
        index.sites.last_mut().expect("Inserted Cookie site")
    };
    if snapshot_ms < entry.last_browser_snapshot_unix_ms {
        record_skipped("older-snapshot");
        return Ok(());
    }
    // Browser synchronization owns managed site files, never external paths.
    if entry.cookie_file != file_name {
        return Err("Automatic Cookie save target is not a managed site file.".to_owned());
    }
    let target = cookie_dir.join(&entry.cookie_file);
    let merged = merge_records(&target, &incoming, &root)?;
    entry.login_url = url.to_owned();
    entry.auto_update_from_extension = true;
    entry.last_browser_snapshot_unix_ms = snapshot_ms;
    update_record_metadata(entry, &merged);
    write_records(&target, &merged)?;
    write_cookie_site_index(&cookie_dir, &index)?;
    record_saved(true, incoming.len());
    Ok(())
}

fn request_site(request: &Value) -> Result<(&str, String, String), String> {
    let url = request["url"]
        .as_str()
        .ok_or("Browser cookie request URL is invalid.")?;
    let root = site_domain_for_url(url)?;
    let id = site_id_from_domain(&root);
    if id.is_empty() {
        return Err("Could not derive a cookie site identifier.".to_owned());
    }
    Ok((url, root, id))
}

fn incoming_records(request: &Value) -> Result<Vec<CookieRecord>, String> {
    let Some(cookies) = NativeCookies::from_request(request)? else {
        return Ok(Vec::new());
    };
    let text = fs::read_to_string(cookies.path())
        .map_err(|_| "Could not read browser Cookie snapshot.")?;
    parse_records(&text)
}

#[derive(Clone)]
struct CookieRecord {
    line: String,
    domain: String,
    path: String,
    name: String,
}

impl CookieRecord {
    fn key(&self) -> (String, String, String) {
        (
            self.domain.trim_start_matches('.').to_ascii_lowercase(),
            self.path.clone(),
            self.name.clone(),
        )
    }
}

fn parse_records(text: &str) -> Result<Vec<CookieRecord>, String> {
    text.lines()
        .filter_map(|raw| {
            let line = raw.strip_prefix("#HttpOnly_").unwrap_or(raw);
            if line.trim().is_empty() || line.starts_with('#') {
                return None;
            }
            Some((|| {
                let fields = line.splitn(7, '\t').collect::<Vec<_>>();
                if fields.len() != 7
                    || !fields[2].starts_with('/')
                    || fields[0].trim_matches('.').is_empty()
                    || !matches!(fields[1], "TRUE" | "FALSE")
                    || !matches!(fields[3], "TRUE" | "FALSE")
                    || (!fields[4].is_empty() && fields[4].parse::<i64>().is_err())
                {
                    return Err("Saved Cookie file is invalid; it was not overwritten.".to_owned());
                }
                Ok(CookieRecord {
                    line: raw.to_owned(),
                    domain: fields[0].to_ascii_lowercase(),
                    path: fields[2].to_owned(),
                    name: fields[5].to_owned(),
                })
            })())
        })
        .collect()
}

fn merge_records(
    target: &Path,
    incoming: &[CookieRecord],
    site: &str,
) -> Result<Vec<CookieRecord>, String> {
    let text = match fs::read_to_string(target) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(_) => {
            return Err("Could not read saved Cookie file; it was not overwritten.".to_owned());
        }
    };
    let keys = incoming
        .iter()
        .map(CookieRecord::key)
        .collect::<BTreeSet<_>>();
    let mut records = parse_records(&text)?
        .into_iter()
        .filter(|cookie| !domain_is_in_site(&cookie.domain, site) && !keys.contains(&cookie.key()))
        .collect::<Vec<_>>();
    records.extend_from_slice(incoming);
    Ok(records)
}

fn write_records(target: &Path, records: &[CookieRecord]) -> Result<(), String> {
    let mut text = String::from("# Netscape HTTP Cookie File\n");
    for cookie in records {
        text.push_str(&cookie.line);
        text.push('\n');
    }
    write_cookie_store_file_atomically(target, text.as_bytes())
}

fn update_record_metadata(entry: &mut CookieSiteIndexEntry, records: &[CookieRecord]) {
    entry.cookie_domains = records
        .iter()
        .map(|cookie| cookie.domain.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    entry.updated_unix = now_ms() / 1000;
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn snapshot_timestamp(request: &Value) -> Option<u64> {
    request["browserCookieContext"]["snapshotUnixMs"]
        .as_u64()
        .filter(|stamp| *stamp > 0 && *stamp <= now_ms().saturating_add(60_000))
}

fn record_saved(automatic: bool, count: usize) {
    super::operation_diagnostics::record(
        "cookies.saved",
        json!({"automatic": automatic, "cookieCount": count, "status": "saved"}),
    );
}

fn record_skipped(reason: &'static str) {
    super::operation_diagnostics::record("cookies.save.skipped", json!({"reason": reason}));
}
