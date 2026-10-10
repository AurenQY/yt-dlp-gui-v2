//! Bounded thumbnail pixels shared by foreground upload and background generation.
//! Run on the fetch worker, not the UI thread. No disk-cache policy lives here.
use image::{DynamicImage, RgbaImage, imageops};

const BACKGROUND_MAX_EDGE: u32 = 96;

pub(super) fn thumbnail_pixel_bounds(logical: [f32; 2], pixels_per_point: f32) -> Option<[u32; 2]> {
    if !pixels_per_point.is_finite()
        || pixels_per_point <= 0.0
        || logical
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return None;
    }
    let scaled = logical.map(|value| (value * pixels_per_point).ceil());
    if scaled
        .iter()
        .any(|value| !value.is_finite() || *value > 32768.0)
    {
        return None;
    }
    Some(scaled.map(|value| value.max(1.0) as u32))
}

#[derive(Debug)]
pub(super) struct ThumbnailPixels {
    rgba: RgbaImage,
}

impl ThumbnailPixels {
    pub(super) fn from_decoded(image: DynamicImage, bounds: [u32; 2]) -> Self {
        // Resize before converting to RGBA so large RGB sources do not also need
        // a full-resolution RGBA allocation. Small images are never enlarged.
        let [width, height] = bounds.map(|value| value.max(1));
        let image = if image.width() > width || image.height() > height {
            image.resize(width, height, imageops::FilterType::Triangle)
        } else {
            image
        };
        Self {
            rgba: image.to_rgba8(),
        }
    }

    pub(super) fn rgba(&self) -> &RgbaImage {
        &self.rgba
    }

    pub(super) fn foreground(&self) -> eframe::egui::ColorImage {
        eframe::egui::ColorImage::from_rgba_unmultiplied(
            [self.rgba.width() as usize, self.rgba.height() as usize],
            self.rgba.as_flat_samples().as_slice(),
        )
    }

    pub(super) fn blurred_background(&self) -> eframe::egui::ColorImage {
        // imageops::thumbnail uses exact dimensions, unlike DynamicImage::thumbnail.
        // Calculate the bounded aspect-preserving dimensions before invoking it.
        let denominator = self
            .rgba
            .width()
            .max(self.rgba.height())
            .max(BACKGROUND_MAX_EDGE);
        let width = (self.rgba.width() * BACKGROUND_MAX_EDGE / denominator).max(1);
        let height = (self.rgba.height() * BACKGROUND_MAX_EDGE / denominator).max(1);
        let reduced = imageops::thumbnail(&self.rgba, width, height);
        let blurred = imageops::blur(&reduced, 3.0);
        eframe::egui::ColorImage::from_rgba_unmultiplied(
            [blurred.width() as usize, blurred.height() as usize],
            blurred.as_flat_samples().as_slice(),
        )
    }
}
