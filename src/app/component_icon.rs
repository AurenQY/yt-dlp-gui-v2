//! Embedded official project icons for the About component pool.
//!
//! The UI never fetches these at runtime. Source URLs and provenance are kept
//! beside the assets in `assets/component-icons/SOURCES.md`.

use eframe::egui::{self, Image};

use crate::infrastructure::ManagedComponentId;

pub(crate) fn component_icon_image(id: ManagedComponentId, size: f32) -> Image<'static> {
    let (name, extension, bytes): (&str, &str, &'static [u8]) = match id {
        ManagedComponentId::App => (
            "yt-dlp-gui-v2",
            "ico",
            include_bytes!("../../assets/logo.ico"),
        ),
        ManagedComponentId::YtDlp => (
            "yt-dlp",
            "ico",
            include_bytes!("../../assets/component-icons/yt-dlp.ico"),
        ),
        ManagedComponentId::GalleryDl => (
            "gallery-dl",
            "png",
            include_bytes!("../../assets/component-icons/gallery-dl.png"),
        ),
        ManagedComponentId::Ffmpeg => (
            "ffmpeg",
            "ico",
            include_bytes!("../../assets/component-icons/ffmpeg.ico"),
        ),
        ManagedComponentId::Deno => (
            "deno",
            "svg",
            include_bytes!("../../assets/component-icons/deno.svg"),
        ),
        ManagedComponentId::Aria2c => (
            "aria2",
            "png",
            include_bytes!("../../assets/component-icons/aria2.png"),
        ),
    };

    Image::from_bytes(format!("bytes://component-icons/{name}.{extension}"), bytes)
        .fit_to_exact_size(egui::vec2(size, size))
}
