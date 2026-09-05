use super::{AppMode, AppState, FormatPickerViewMode};
use crate::infrastructure::FormatPickerViewPreference;

impl AppState {
    pub fn options_root_scroll_offset(&self) -> f32 {
        self.config.options_scroll_offset
    }

    pub fn remember_options_root_scroll_offset(&mut self, offset: f32) {
        self.config.options_scroll_offset = normalized_scroll_offset(offset);
    }

    pub fn advance_root_scroll_offset(&self) -> f32 {
        self.config.advance_scroll_offset
    }

    pub fn remember_advance_root_scroll_offset(&mut self, offset: f32) {
        self.config.advance_scroll_offset = normalized_scroll_offset(offset);
    }

    pub fn preferred_format_picker_view_mode(&self) -> FormatPickerViewMode {
        resolved_format_picker_view_mode(self.config.format_picker_view_preference, self.app_mode())
    }

    pub fn remember_format_picker_view_mode(&mut self, mode: FormatPickerViewMode) {
        self.config.format_picker_view_preference = Some(match mode {
            FormatPickerViewMode::Filter => FormatPickerViewPreference::Filter,
            FormatPickerViewMode::Table => FormatPickerViewPreference::Table,
        });
    }

    pub fn save_persisted_ui_view_state(&self) {
        let _ = self.config.save();
    }
}

fn normalized_scroll_offset(offset: f32) -> f32 {
    if offset.is_finite() {
        offset.max(0.0)
    } else {
        0.0
    }
}

fn resolved_format_picker_view_mode(
    preference: Option<FormatPickerViewPreference>,
    app_mode: AppMode,
) -> FormatPickerViewMode {
    match preference {
        Some(FormatPickerViewPreference::Filter) => FormatPickerViewMode::Filter,
        Some(FormatPickerViewPreference::Table) => FormatPickerViewMode::Table,
        None if app_mode == AppMode::Origin => FormatPickerViewMode::Table,
        None => FormatPickerViewMode::Filter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_scroll_offsets_reject_negative_and_non_finite_values() {
        assert_eq!(normalized_scroll_offset(-1.0), 0.0);
        assert_eq!(normalized_scroll_offset(f32::NAN), 0.0);
        assert_eq!(normalized_scroll_offset(f32::INFINITY), 0.0);
        assert_eq!(normalized_scroll_offset(123.5), 123.5);
    }

    #[test]
    fn picker_view_uses_mode_default_until_a_user_preference_exists() {
        assert_eq!(
            resolved_format_picker_view_mode(None, AppMode::Origin),
            FormatPickerViewMode::Table
        );
        assert_eq!(
            resolved_format_picker_view_mode(None, AppMode::Standard),
            FormatPickerViewMode::Filter
        );
        assert_eq!(
            resolved_format_picker_view_mode(
                Some(FormatPickerViewPreference::Table),
                AppMode::Standard,
            ),
            FormatPickerViewMode::Table
        );
        assert_eq!(
            resolved_format_picker_view_mode(
                Some(FormatPickerViewPreference::Filter),
                AppMode::Origin,
            ),
            FormatPickerViewMode::Filter
        );
    }
}
