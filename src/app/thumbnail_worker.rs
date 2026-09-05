use std::io::Read;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

const THUMBNAIL_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_THUMBNAIL_BYTES: u64 = 12 * 1024 * 1024;

#[cfg(test)]
#[path = "thumbnail_worker_tests.rs"]
mod tests;

pub struct ThumbnailFetchEvent {
    pub key: String,
    pub attempt: u8,
    pub retryable: bool,
    pub result: Result<eframe::egui::ColorImage, String>,
}

pub fn run_thumbnail_fetch_worker(
    key: String,
    urls: Vec<String>,
    proxy_url: String,
    no_check_certificates: bool,
    attempt: u8,
    result_tx: Sender<ThumbnailFetchEvent>,
) {
    thread::spawn(move || {
        let result = fetch_thumbnail_candidates(&urls, &proxy_url, no_check_certificates);
        let retryable = result.as_ref().err().is_some_and(|error| error.retryable);
        let _ = result_tx.send(ThumbnailFetchEvent {
            key,
            attempt,
            retryable,
            result: result.map_err(|error| error.message),
        });
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
) -> Result<eframe::egui::ColorImage, ThumbnailFailure> {
    let mut last_error = ThumbnailFailure::terminal("Thumbnail load failed: no candidates");
    // Bound work for sites returning large thumbnail lists. Network failures retry
    // the request later; only a missing or undecodable image advances candidates.
    for url in urls.iter().take(8) {
        match fetch_thumbnail_image(url, proxy_url, no_check_certificates) {
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
) -> Result<eframe::egui::ColorImage, ThumbnailFailure> {
    let bytes = fetch_thumbnail_bytes_result(url, proxy_url, no_check_certificates)?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| ThumbnailFailure {
            message: format!("Thumbnail decode failed: {error}"),
            retryable: false,
            try_next_candidate: true,
        })?
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.as_flat_samples();
    Ok(eframe::egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
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

    let mut builder = ureq::Agent::config_builder()
        .timeout_global(Some(THUMBNAIL_TIMEOUT))
        .user_agent("yt-dlp-gui-v2 thumbnail loader");

    let proxy_url = proxy_url.trim();
    if !proxy_url.is_empty() {
        let proxy = ureq::Proxy::new(proxy_url)
            .map_err(|_| ThumbnailFailure::terminal("Invalid thumbnail proxy setting"))?;
        builder = builder.proxy(Some(proxy));
    }

    if no_check_certificates {
        builder = builder.tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::Rustls)
                .disable_verification(true)
                .build(),
        );
    }

    let agent = builder.build().new_agent();
    let mut response = agent.get(url).call().map_err(format_ureq_error)?;

    let mut reader = response
        .body_mut()
        .as_reader()
        .take(MAX_THUMBNAIL_BYTES + 1);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|error| ThumbnailFailure {
            message: format!("Thumbnail load failed: {error}"),
            retryable: true,
            try_next_candidate: false,
        })?;

    if bytes.is_empty() {
        return Err(ThumbnailFailure {
            message: "Thumbnail load failed: no data received".to_owned(),
            retryable: true,
            try_next_candidate: false,
        });
    }
    if bytes.len() as u64 > MAX_THUMBNAIL_BYTES {
        return Err(ThumbnailFailure::terminal(
            "Thumbnail load failed: image too large",
        ));
    }

    Ok(bytes)
}

fn format_ureq_error(error: ureq::Error) -> ThumbnailFailure {
    match error {
        ureq::Error::StatusCode(code) => ThumbnailFailure {
            message: format!("Thumbnail load failed: HTTP {code}"),
            retryable: matches!(code, 408 | 429 | 500..=599),
            try_next_candidate: matches!(code, 404 | 410),
        },
        error => ThumbnailFailure {
            message: format!("Thumbnail load failed: {error}"),
            retryable: true,
            try_next_candidate: false,
        },
    }
}
