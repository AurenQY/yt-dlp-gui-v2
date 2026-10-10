//! UI wording belongs here; the domain range holds only intent and validation.
use crate::domain::GalleryDownloadRange;
use crate::i18n::{Language, format_text, text};

pub(super) fn summary(range: &GalleryDownloadRange, language: Language) -> String {
    match range {
        GalleryDownloadRange::Posts { count } => format_text(
            language,
            "gallery.range.latest",
            &[("{count}", &count.to_string())],
        ),
        GalleryDownloadRange::Recent { days: 1, .. } => {
            text(language, "gallery.range.recent24h").to_owned()
        }
        GalleryDownloadRange::Recent { days, .. } => format_text(
            language,
            "gallery.range.days",
            &[("{days}", &days.to_string())],
        ),
        GalleryDownloadRange::Since { date } => {
            format_text(language, "gallery.range.since", &[("{date}", date)])
        }
        GalleryDownloadRange::All => text(language, "item.all").to_owned(),
    }
}
