use serde_json::Value;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) struct NativeCookies {
    path: PathBuf,
    youtube_authenticated: bool,
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
        let host = request["url"]
            .as_str()
            .unwrap_or("")
            .split_once("://")
            .map(|(_, rest)| rest.split(['/', '?', '#']).next().unwrap_or(""))
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        let mut text = String::from("# Netscape HTTP Cookie File\n");
        let mut has_login_info = false;
        let mut has_sapisid = false;
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
            if bare.is_empty()
                || (host != bare && (host_only || !host.ends_with(&format!(".{bare}"))))
            {
                return Err("Browser cookie domain does not match the target.".into());
            }
            let path = field("path")?;
            if !path.starts_with('/') {
                return Err("Invalid browser cookie path.".into());
            }
            let expiry = cookie["expirationDate"].as_f64().unwrap_or(0.0).max(0.0) as u64;
            let name = field("name")?;
            has_login_info |= name == "LOGIN_INFO";
            has_sapisid |= matches!(name, "SAPISID" | "__Secure-1PAPISID" | "__Secure-3PAPISID");
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
            youtube_authenticated: has_login_info && has_sapisid,
        };
        file.write_all(text.as_bytes())
            .map_err(|_| "Could not write temporary browser cookies.")?;
        file.flush()
            .map_err(|_| "Could not flush temporary browser cookies.")?;
        Ok(Some(guard))
    }

    pub(super) fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub(super) fn youtube_authenticated(&self) -> bool {
        self.youtube_authenticated
    }

    pub(super) fn into_persisted_path(mut self) -> PathBuf {
        std::mem::take(&mut self.path)
    }
}

impl Drop for NativeCookies {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cookies_are_scoped_and_deleted_after_use() {
        let request = serde_json::json!({"url":"https://example.com/video","browserCookies":[{
            "domain":".example.com","hostOnly":false,"path":"/","secure":true,"name":"test","value":"synthetic"
        }]});
        let guard = NativeCookies::from_request(&request).unwrap().unwrap();
        let path = guard.path();
        assert!(path.is_file());
        drop(guard);
        assert!(!path.exists());
        let mut wrong = request.clone();
        wrong["browserCookies"][0]["domain"] = serde_json::json!(".unrelated.com");
        assert!(NativeCookies::from_request(&wrong).is_err());
        let mut invalid = request;
        invalid["browserCookies"][0]["value"] = serde_json::json!("bad\nline");
        assert!(NativeCookies::from_request(&invalid).is_err());
    }

    #[test]
    fn youtube_login_requires_login_info_and_a_sapisid_cookie() {
        let request = serde_json::json!({
            "url":"https://www.youtube.com/watch?v=video",
            "browserCookies":[
                {"domain":".youtube.com","hostOnly":false,"path":"/","secure":true,"name":"LOGIN_INFO","value":"login"},
                {"domain":".youtube.com","hostOnly":false,"path":"/","secure":true,"name":"__Secure-3PAPISID","value":"sid"}
            ]
        });
        let guard = NativeCookies::from_request(&request).unwrap().unwrap();
        assert!(guard.youtube_authenticated());

        let mut incomplete = request;
        incomplete["browserCookies"].as_array_mut().unwrap().pop();
        let guard = NativeCookies::from_request(&incomplete).unwrap().unwrap();
        assert!(!guard.youtube_authenticated());
    }
}
