//! Shared image-only paint layers for the queue card's allocated thumbnail slot.
//! The card template owns the slot; this module never positions sibling controls.

use eframe::egui::{self, Color32, Rect, Ui, Vec2};
use std::time::{Duration, Instant};

use crate::app::state::{ThumbnailReadyImage, ThumbnailRenderSource};

const FOREGROUND_FADE: Duration = Duration::from_millis(300);
const BACKGROUND_FADE: Duration = Duration::from_millis(450);

pub(super) fn paint_thumbnail_image_layers(
    ui: &Ui,
    slot: Rect,
    source: &ThumbnailRenderSource,
    url: &str,
) -> bool {
    match source {
        ThumbnailRenderSource::Texture(texture) => {
            paint_texture_layers(ui, slot, texture);
            true
        }
        ThumbnailRenderSource::DirectUrl if !url.trim().is_empty() => {
            paint_direct_url_layers(ui, slot, url);
            true
        }
        _ => false,
    }
}

fn paint_texture_layers(ui: &Ui, slot: Rect, ready: &ThumbnailReadyImage) {
    let now = Instant::now();
    if let (Some(background), Some(start)) =
        (ready.blurred_background.as_ref(), ready.background_ready_at)
    {
        let alpha = fade_alpha(now, start, BACKGROUND_FADE);
        if alpha < u8::MAX {
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
        if alpha > 0 {
            ui.painter().image(
                background.id(),
                slot,
                thumbnail_cover_uv_rect(slot.size(), background.size_vec2()),
                Color32::from_white_alpha(alpha),
            );
            paint_background_dimmer(ui, slot, alpha);
        }
    }

    let foreground_alpha = fade_alpha(now, ready.foreground_ready_at, FOREGROUND_FADE);
    if foreground_alpha < u8::MAX {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
    ui.painter().image(
        ready.foreground.id(),
        thumbnail_contain_rect(slot, ready.foreground.size_vec2()),
        full_image_uv_rect(),
        Color32::from_white_alpha(foreground_alpha),
    );
}

fn paint_direct_url_layers(ui: &Ui, slot: Rect, url: &str) {
    let image = egui::Image::new(url)
        .fit_to_exact_size(slot.size())
        .maintain_aspect_ratio(true)
        .show_loading_spinner(false);
    let contained_size = image
        .load_and_calc_size(ui, slot.size())
        .unwrap_or(slot.size());
    // Non-network sources bypass the thumbnail worker, so no decoded pixels
    // are available for asynchronous blur. Keep their foreground unstretched.
    image.paint_at(ui, Rect::from_center_size(slot.center(), contained_size));
}

fn paint_background_dimmer(ui: &Ui, slot: Rect, background_alpha: u8) {
    // The foreground stays untouched. Light themes need a stronger darkening
    // layer so the cropped background does not become visually dominant.
    let alpha: u8 = if ui.visuals().dark_mode { 150 } else { 185 };
    let alpha = (u16::from(alpha) * u16::from(background_alpha) / 255) as u8;
    ui.painter()
        .rect_filled(slot, 0.0, Color32::from_black_alpha(alpha));
}

fn fade_alpha(now: Instant, start: Instant, duration: Duration) -> u8 {
    let elapsed = now.saturating_duration_since(start);
    let progress = (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0);
    let eased = 1.0 - (1.0 - progress).powi(2);
    (eased * 255.0).round() as u8
}

fn full_image_uv_rect() -> Rect {
    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
}

/// Aspect-fill with centered cropping; only texture coordinates change.
fn thumbnail_cover_uv_rect(slot_size: Vec2, source_size: Vec2) -> Rect {
    if slot_size.x <= 0.0 || slot_size.y <= 0.0 || source_size.x <= 0.0 || source_size.y <= 0.0 {
        return full_image_uv_rect();
    }
    let slot_ratio = slot_size.x / slot_size.y;
    let source_ratio = source_size.x / source_size.y;
    if !slot_ratio.is_finite() || !source_ratio.is_finite() {
        return full_image_uv_rect();
    }
    if source_ratio > slot_ratio {
        let margin = (1.0 - slot_ratio / source_ratio) * 0.5;
        Rect::from_min_max(egui::pos2(margin, 0.0), egui::pos2(1.0 - margin, 1.0))
    } else {
        let margin = (1.0 - source_ratio / slot_ratio) * 0.5;
        Rect::from_min_max(egui::pos2(0.0, margin), egui::pos2(1.0, 1.0 - margin))
    }
}

/// Aspect-fit inside the existing template slot; sibling layout is unchanged.
fn thumbnail_contain_rect(slot: Rect, source_size: Vec2) -> Rect {
    if slot.width() <= 0.0
        || slot.height() <= 0.0
        || !source_size.x.is_finite()
        || !source_size.y.is_finite()
        || source_size.x <= 0.0
        || source_size.y <= 0.0
    {
        return slot;
    }
    let scale = (slot.width() / source_size.x).min(slot.height() / source_size.y);
    Rect::from_center_size(slot.center(), source_size * scale)
}
