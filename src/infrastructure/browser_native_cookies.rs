use serde_json::Value;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use super::cookie_site_scope::{domain_is_in_site, site_domain_for_url};

pub(super) struct NativeCookies {
    path: PathBuf,
    cookie_count: usize,
}

impl NativeCookies {
    pub(super) fn from_request(request: &Value) -> Result<Option<Self>, String> {
        let Some(cookies) = request.get("browserCookies").and_then(Value::as_array) else {
            return Ok(None);
        };
        if cookies.is_empty() {
            return Ok(None);
        }
        if cookies.len() > 512 {
            return Err("Too many browser cookies.".into());
        }
        let site = site_domain_for_url(request["url"].as_str().unwrap_or(""))?;
        let mut text = String::from("# Netscape HTTP Cookie File\n");
        for cookie in cookies {
            let field = |key: &str| -> Result<&str, String> {
                let value = cookie[key].as_str().ok_or("Invalid browser cookie.")?;
                if value.chars().any(char::is_control) {
                    return Err("Invalid browser cookie.".into());
                }
                Ok(value)
            };
            let domain = field("domain")?;
            let bare = domain.trim_start_matches('.').to_ascii_lowercase();
            let host_only = cookie["hostOnly"].as_bool().unwrap_or(true);
            if bare.is_empty() || !domain_is_in_site(&bare, &site) {
                return Err("Browser cookie domain does not match the website.".into());
            }
            let path = field("path")?;
            if !path.starts_with('/') {
                return Err("Invalid browser cookie path.".into());
            }
            let expiry = cookie["expirationDate"].as_f64().unwrap_or(0.0).max(0.0) as u64;
            let name = field("name")?;
            text.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                domain,
                if host_only { "FALSE" } else { "TRUE" },
                path,
                if cookie["secure"].as_bool().unwrap_or(false) {
                    "TRUE"
                } else {
                    "FALSE"
                },
                expiry,
                name,
                field("value")?
            ));
        }
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Clock unavailable.")?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "yt-dlp-native-{}-{stamp}.cookies",
            std::process::id()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|_| "Could not create temporary browser cookies.")?;
        let guard = Self {
            path,
            cookie_count: cookies.len(),
        };
        file.write_all(text.as_bytes())
            .map_err(|_| "Could not write temporary browser cookies.")?;
        file.flush()
            .map_err(|_| "Could not flush temporary browser cookies.")?;
        super::operation_diagnostics::record(
            "cookies.resolved",
            serde_json::json!({"cookieCount": guard.cookie_count, "source": "browser-storage-snapshot"}),
        );
        Ok(Some(guard))
    }

    pub(super) fn path(&self) -> PathBuf {
        self.path.clone()
    }
}

/// Browser-only anonymous client coverage must inspect the selected saved
/// source, not browser credentials that analysis no longer consumes directly.
pub(super) fn shared_source_allows_anonymous_youtube_clients(
    tools: &super::ToolPaths,
    use_cookies: bool,
    url: &str,
) -> Result<bool, String> {
    let site = site_domain_for_url(url)?;
    if site != "youtube.com" && site != "youtu.be" {
        return Ok(true);
    }
    let source = tools.resolve_shared_cookie_source(use_cookies, Some(url))?;
    let path = match source {
        None => return Ok(true),
        // Authentication is unknown for browser extraction; don't force anonymous clients.
        Some(super::tools::ResolvedCookieSource::Browser(_)) => return Ok(false),
        Some(super::tools::ResolvedCookieSource::File(path)) => path,
    };
    let text = fs::read_to_string(path).map_err(|_| "Could not read selected Cookie file.")?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let mut login_info = false;
    let mut sapisid = false;
    for raw in text.lines() {
        let line = raw.strip_prefix("#HttpOnly_").unwrap_or(raw);
        if line.starts_with('#') {
            continue;
        }
        let fields = line.splitn(7, '\t').collect::<Vec<_>>();
        if fields.len() != 7 {
            continue;
        }
        let domain = fields[0].trim_start_matches('.').to_ascii_lowercase();
        let expiry = fields[4].parse::<u64>().unwrap_or_default();
        if (domain != "youtube.com" && !domain.ends_with(".youtube.com"))
            || (expiry != 0 && expiry <= now)
        {
            continue;
        }
        login_info |= fields[5] == "LOGIN_INFO";
        sapisid |= matches!(
            fields[5],
            "SAPISID" | "__Secure-1PAPISID" | "__Secure-3PAPISID"
        );
    }
    Ok(!(login_info && sapisid))
}

impl Drop for NativeCookies {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
