use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use super::thumbnail_pixels::ThumbnailPixels;
use crate::infrastructure::async_http::{self, ClientOptions};
use crate::infrastructure::queue_thumbnail_cache::QueueThumbnailCacheRequest;

const THUMBNAIL_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_THUMBNAIL_BYTES: u64 = 12 * 1024 * 1024;

pub struct ThumbnailFetchEvent {
    pub key: String,
    pub attempt: u8,
    pub retryable: bool,
    pub stage: ThumbnailFetchStage,
    pub result: Result<eframe::egui::ColorImage, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThumbnailFetchStage {
    Foreground,
    BlurredBackground,
}

pub struct ThumbnailFetchOptions {
    pub bounds: [u32; 2],
    pub queue_cache: Option<QueueThumbnailCacheRequest>,
    pub blur_background: bool,
}

pub fn run_thumbnail_fetch_worker(
    key: String,
    urls: Vec<String>,
    proxy_url: String,
    no_check_certificates: bool,
    attempt: u8,
    options: ThumbnailFetchOptions,
    result_tx: Sender<ThumbnailFetchEvent>,
) {
    thread::spawn(move || {
        let cached =
            options
                .queue_cache
                .as_ref()
                .and_then(|request| match request.cache.load(request) {
                    Ok(image) => image,
                    Err(error) => {
                        eprintln!("[thumbnail-cache] {error}");
                        None
                    }
                });
        let mut fetched = cached.is_none();
        let decoded = if let Some(image) = cached {
            Ok(ThumbnailPixels::from_decoded(image, options.bounds))
        } else {
            match fetch_thumbnail_candidates(
                &urls,
                &proxy_url,
                no_check_certificates,
                options.bounds,
            ) {
                Ok(image) => Ok(image),
                Err(error) => {
                    let fallback = options
                        .queue_cache
                        .as_ref()
                        .and_then(|request| request.cache.load_fallback(request).ok().flatten());
                    if let Some(image) = fallback {
                        fetched = false;
                        Ok(ThumbnailPixels::from_decoded(image, options.bounds))
                    } else {
                        Err(error)
                    }
                }
            }
        };
        match decoded {
            Ok(decoded) => {
                if result_tx
                    .send(ThumbnailFetchEvent {
                        key: key.clone(),
                        attempt,
                        retryable: false,
                        stage: ThumbnailFetchStage::Foreground,
                        result: Ok(decoded.foreground()),
                    })
                    .is_ok()
                {
                    if options.blur_background {
                        let _ = result_tx.send(ThumbnailFetchEvent {
                            key,
                            attempt,
                            retryable: false,
                            stage: ThumbnailFetchStage::BlurredBackground,
                            result: Ok(decoded.blurred_background()),
                        });
                    }
                    if fetched {
                        if let Some(request) = options.queue_cache.as_ref() {
                            if let Err(error) = request.cache.store(request, decoded.rgba()) {
                                eprintln!("[thumbnail-cache] {error}");
                            }
                        }
                    }
                }
            }
            Err(error) => {
                let _ = result_tx.send(ThumbnailFetchEvent {
                    key,
                    attempt,
                    retryable: error.retryable,
                    stage: ThumbnailFetchStage::Foreground,
                    result: Err(error.message),
                });
            }
        }
    });
}

#[derive(Debug)]
struct ThumbnailFailure {
    message: String,
    retryable: bool,
    try_next_candidate: bool,
}

impl ThumbnailFailure {
    fn terminal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
            try_next_candidate: false,
        }
    }
}

pub(super) fn thumbnail_retry_delay(attempt: u8) -> Option<Duration> {
    match attempt {
        1 => Some(Duration::from_secs(2)),
        2 => Some(Duration::from_secs(10)),
        _ => None,
    }
}

fn fetch_thumbnail_candidates(
    urls: &[String],
    proxy_url: &str,
    no_check_certificates: bool,
    bounds: [u32; 2],
) -> Result<ThumbnailPixels, ThumbnailFailure> {
    let mut last_error = ThumbnailFailure::terminal("Thumbnail load failed: no candidates");
    // Bound work for sites returning large thumbnail lists. Network failures retry
    // the request later; only a missing or undecodable image advances candidates.
    for url in urls.iter().take(8) {
        match fetch_thumbnail_image(url, proxy_url, no_check_certificates, bounds) {
            Ok(image) => return Ok(image),
            Err(error) if error.try_next_candidate => last_error = error,
            Err(error) => return Err(error),
        }
    }
    Err(last_error)
}

fn fetch_thumbnail_image(
    url: &str,
    proxy_url: &str,
    no_check_certificates: bool,
    bounds: [u32; 2],
) -> Result<ThumbnailPixels, ThumbnailFailure> {
    let bytes = fetch_thumbnail_bytes_result(url, proxy_url, no_check_certificates)?;
    let image = image::load_from_memory(&bytes).map_err(|error| ThumbnailFailure {
        message: format!("Thumbnail decode failed: {error}"),
        retryable: false,
        try_next_candidate: true,
    })?;
    Ok(ThumbnailPixels::from_decoded(image, bounds))
}

pub(crate) fn fetch_thumbnail_bytes(
    url: &str,
    proxy_url: &str,
    no_check_certificates: bool,
) -> Result<Vec<u8>, String> {
    fetch_thumbnail_bytes_result(url, proxy_url, no_check_certificates)
        .map_err(|error| error.message)
}

fn fetch_thumbnail_bytes_result(
    url: &str,
    proxy_url: &str,
    no_check_certificates: bool,
) -> Result<Vec<u8>, ThumbnailFailure> {
    let url = url.trim();
    if url.is_empty() {
        return Err(ThumbnailFailure::terminal(
            "Thumbnail load failed: empty URL",
        ));
    }

    let mut options = ClientOptions::new("yt-dlp-gui-v2 thumbnail loader");
    options.proxy_url = (!proxy_url.trim().is_empty()).then_some(proxy_url.trim());
    options.connect_timeout = THUMBNAIL_TIMEOUT;
    options.read_timeout = THUMBNAIL_TIMEOUT;
    options.danger_accept_invalid_certs = no_check_certificates;
    let client = async_http::build_client(options).map_err(|message| ThumbnailFailure {
        message,
        retryable: false,
        try_next_candidate: false,
    })?;

    async_http::block_on(async {
        let mut response = async_http::send_get(&client, url, [])
            .await
            .map_err(|message| ThumbnailFailure {
                message: format!("Thumbnail load failed: {message}"),
                retryable: true,
                try_next_candidate: false,
            })?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(format_http_status_error(status));
        }

        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| ThumbnailFailure {
            message: format!("Thumbnail load failed: {error}"),
            retryable: true,
            try_next_candidate: false,
        })? {
            if bytes.len().saturating_add(chunk.len()) > MAX_THUMBNAIL_BYTES as usize {
                return Err(ThumbnailFailure::terminal(
                    "Thumbnail load failed: image too large",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }

        if bytes.is_empty() {
            return Err(ThumbnailFailure {
                message: "Thumbnail load failed: no data received".to_owned(),
                retryable: true,
                try_next_candidate: false,
            });
        }
        Ok(bytes)
    })
}

fn format_http_status_error(code: u16) -> ThumbnailFailure {
    ThumbnailFailure {
        message: format!("Thumbnail load failed: HTTP {code}"),
        retryable: matches!(code, 408 | 429 | 500..=599),
        try_next_candidate: matches!(code, 404 | 410),
    }
}
