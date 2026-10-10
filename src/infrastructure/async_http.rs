use std::collections::HashMap;
use std::future::Future;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use reqwest::{Client, ClientBuilder, Response};
use serde::de::DeserializeOwned;
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};

const DEFAULT_USER_AGENT: &str = "yt-dlp-gui-v2";

static HTTP_RUNTIME: OnceLock<Runtime> = OnceLock::new();
static HTTP_TLS_PROVIDER: OnceLock<()> = OnceLock::new();
static HTTP_CLIENTS: OnceLock<Mutex<HashMap<ClientKey, Client>>> = OnceLock::new();

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ClientKey {
    user_agent: String,
    proxy_url: Option<String>,
    bypass_proxy: bool,
    connect_timeout: Duration,
    read_timeout: Duration,
    danger_accept_invalid_certs: bool,
}

fn runtime() -> &'static Runtime {
    HTTP_RUNTIME.get_or_init(|| {
        RuntimeBuilder::new_multi_thread()
            .worker_threads(2)
            .thread_name("yt-dlp-gui-http")
            .enable_all()
            .build()
            .expect("failed to create async HTTP runtime")
    })
}

pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    runtime().block_on(future)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ClientOptions<'a> {
    pub user_agent: &'a str,
    pub proxy_url: Option<&'a str>,
    /// Direct local requests must not use explicit, environment or system proxies.
    pub bypass_proxy: bool,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub danger_accept_invalid_certs: bool,
}

impl<'a> ClientOptions<'a> {
    pub(crate) fn new(user_agent: &'a str) -> Self {
        Self {
            user_agent,
            proxy_url: None,
            bypass_proxy: false,
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(20),
            danger_accept_invalid_certs: false,
        }
    }
}

pub(crate) fn build_client(options: ClientOptions<'_>) -> Result<Client, String> {
    let user_agent = if options.user_agent.trim().is_empty() {
        DEFAULT_USER_AGENT
    } else {
        options.user_agent.trim()
    };
    let proxy_url = options
        .proxy_url
        .filter(|_| !options.bypass_proxy)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let key = ClientKey {
        user_agent: user_agent.to_owned(),
        proxy_url: proxy_url.clone(),
        bypass_proxy: options.bypass_proxy,
        connect_timeout: options.connect_timeout,
        read_timeout: options.read_timeout,
        danger_accept_invalid_certs: options.danger_accept_invalid_certs,
    };

    let clients = HTTP_CLIENTS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(client) = clients
        .lock()
        .map_err(|_| "HTTP client cache lock is poisoned".to_owned())?
        .get(&key)
        .cloned()
    {
        return Ok(client);
    }

    // Reqwest's rustls-no-provider requires a process default before Client::build.
    // Keep one ring backend rather than also linking AWS-LC.
    HTTP_TLS_PROVIDER.get_or_init(|| {
        // Another client may already have installed the process-wide provider.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
    let mut builder = ClientBuilder::new()
        .user_agent(user_agent)
        .connect_timeout(options.connect_timeout)
        .read_timeout(options.read_timeout)
        .danger_accept_invalid_certs(options.danger_accept_invalid_certs);

    if options.bypass_proxy {
        builder = builder.no_proxy();
    } else if let Some(proxy_url) = proxy_url.as_deref() {
        let proxy = reqwest::Proxy::all(proxy_url)
            .map_err(|error| format!("Invalid proxy URL {proxy_url}: {error}"))?;
        builder = builder.proxy(proxy);
    }

    let client = builder
        .build()
        .map_err(|error| format!("Could not create HTTP client: {error}"))?;
    clients
        .lock()
        .map_err(|_| "HTTP client cache lock is poisoned".to_owned())?
        .insert(key, client.clone());
    Ok(client)
}

pub(crate) async fn send_get(
    client: &Client,
    url: &str,
    headers: impl IntoIterator<Item = (String, String)>,
) -> Result<Response, String> {
    let mut request = client.get(url);
    for (name, value) in headers {
        let name = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|error| format!("Invalid HTTP header name {name}: {error}"))?;
        let value = reqwest::header::HeaderValue::from_str(&value)
            .map_err(|error| format!("Invalid HTTP header value: {error}"))?;
        request = request.header(name, value);
    }

    request
        .send()
        .await
        .map_err(|error| format!("Could not request {url}: {error}"))
}

pub(crate) async fn get_bytes(
    client: &Client,
    url: &str,
    headers: impl IntoIterator<Item = (String, String)>,
    max_bytes: Option<usize>,
) -> Result<Vec<u8>, String> {
    let mut response = send_get(client, url, headers).await?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {}", status.as_u16()));
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Could not read response from {url}: {error}"))?
    {
        if let Some(limit) = max_bytes {
            if bytes.len().saturating_add(chunk.len()) > limit {
                return Err(format!("response exceeded {limit} bytes"));
            }
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub(crate) async fn get_json<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    headers: impl IntoIterator<Item = (String, String)>,
) -> Result<T, String> {
    let response = send_get(client, url, headers).await?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {}", status.as_u16()));
    }
    response
        .json::<T>()
        .await
        .map_err(|error| format!("Could not parse response from {url}: {error}"))
}

pub(crate) async fn get_text(
    client: &Client,
    url: &str,
    headers: impl IntoIterator<Item = (String, String)>,
) -> Result<String, String> {
    let response = send_get(client, url, headers).await?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {}", status.as_u16()));
    }
    response
        .text()
        .await
        .map_err(|error| format!("Could not read response from {url}: {error}"))
}
