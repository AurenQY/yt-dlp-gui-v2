//! Loopback-only CDP discovery; browser ownership and WebSocket calls stay in the parent.

use std::time::Duration;

use serde_json::Value;

use super::CdpPageTarget;
use crate::infrastructure::async_http::{self, ClientOptions};

const CDP_HTTP_TIMEOUT: Duration = Duration::from_secs(10);

fn read_cdp_response(url: &str, endpoint: &str) -> Result<String, String> {
    let mut options = ClientOptions::new("yt-dlp-gui-cdp");
    options.bypass_proxy = true;
    options.connect_timeout = CDP_HTTP_TIMEOUT;
    options.read_timeout = CDP_HTTP_TIMEOUT;
    let client = async_http::build_client(options)
        .map_err(|error| format!("Could not create CDP HTTP client: {error}"))?;

    let bytes = async_http::block_on(async {
        tokio::time::timeout(
            CDP_HTTP_TIMEOUT,
            async_http::get_bytes(&client, url, [], None),
        )
        .await
        .map_err(|_| format!("{endpoint} request timed out"))?
        .map_err(|error| format!("Could not read {endpoint} response: {error}"))
    })?;
    String::from_utf8(bytes)
        .map_err(|error| format!("Could not read {endpoint} response as UTF-8: {error}"))
}

pub(super) fn read_page_targets(port: u16) -> Result<Vec<CdpPageTarget>, String> {
    let list_url = format!("http://127.0.0.1:{port}/json/list");
    let body = read_cdp_response(&list_url, "CDP target list")?;
    let json: Value = serde_json::from_str(&body)
        .map_err(|error| format!("Could not parse CDP target list response: {error}"))?;
    let Some(targets) = json.as_array() else {
        return Err("CDP target list was not an array.".to_owned());
    };

    let mut page_targets = Vec::new();
    for target in targets {
        let target_type = target
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if target_type != "page" {
            continue;
        }
        let Some(ws_url) = target
            .get("webSocketDebuggerUrl")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        else {
            continue;
        };
        let url = target
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        page_targets.push(CdpPageTarget { ws_url, url });
    }
    Ok(page_targets)
}

pub(super) fn read_cdp_version(version_url: &str) -> Result<Option<String>, String> {
    let body = read_cdp_response(version_url, "CDP version")?;
    let json: Value = serde_json::from_str(&body)
        .map_err(|error| format!("Could not parse CDP version response: {error}"))?;
    let websocket_url = json
        .get("webSocketDebuggerUrl")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    Ok(websocket_url)
}
