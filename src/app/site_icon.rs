//! Offline website branding from one shared asset catalog.
//! This lookup never determines extractor support or video/playlist scope.

use eframe::egui::{self, Image};

#[path = "site_icon_assets.rs"]
mod assets;
pub(crate) use assets::SiteIcon;

pub(crate) fn site_icon_for_url(input: &str) -> Option<SiteIcon> {
    let url = reqwest::Url::parse(input.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    assets::SITE_ICONS.iter().copied().find(|icon| {
        icon.hosts().iter().any(|domain| {
            host == *domain
                || host
                    .strip_suffix(domain)
                    .is_some_and(|prefix| prefix.ends_with('.'))
        })
    })
}

pub(crate) fn site_icon_image(icon: SiteIcon, size: f32) -> Image<'static> {
    Image::from_bytes(format!("bytes://site-icons/{}", icon.file()), icon.bytes())
        .fit_to_exact_size(egui::vec2(size, size))
}

pub(crate) fn yt_dlp_site_icon(extractor_key: &str) -> Option<SiteIcon> {
    let key = extractor_key.trim().to_ascii_lowercase();
    assets::SITE_ICONS.iter().copied().find(|icon| {
        icon.extractor_prefixes()
            .iter()
            .any(|prefix| key.starts_with(prefix))
    })
}
