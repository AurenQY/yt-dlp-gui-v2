use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Color32, RichText, Sense, Ui};

use crate::app::component_icon::component_icon_image;
use crate::app::state::AppState;
use crate::app::widgets::url_input::{AppTextBox, AppTextBoxSyntax};
use crate::infrastructure::{
    ComponentOwnership, ComponentUpdateEntry, ComponentUpdateStatus, DependencyTool,
    ManagedComponentId,
};

use super::common::icon_text_button;
use super::semantic_ui_metrics;
use crate::app::widgets::icon::AppIcon;

// ABOUT PAINTER RULES — this file is not a layout owner.
//
// The About page layout lives in `about_tab_template.rs`.
// Row/cell UiElement placement lives in `about_tab_controls.rs`.
// Functions in this file receive one already allocated slot and only draw the
// widget/text/Markdown for that slot.
//
// Forbidden here:
// - rebuilding component rows with `ui.horizontal` or manual spacer math;
// - owning left/right/center alignment for component columns;
// - creating another taffy/template root;
// - adding local i18n fallback dictionaries;
// - hand-calculated offset/padding fixes such as fixed rect shrink, baseline
//   nudges, or width subtraction.  About cells can be squeezed to zero by the
//   template, so every slot-local size must be derived from allocated Rects and
//   clamped before passing it back into egui.
//
// 每次處理中，如有發現不符合地方應強力修正；尤其不要把欄位對齊退回
// painter 內部；若需要改欄位，先改 about_tab_template.rs / about_tab_controls.rs。

pub(super) fn render_about_tab(ui: &mut Ui, state: &mut AppState) {
    super::about_tab_template::render_about_tab_template(ui, state);
}

pub(super) fn about_component_ids() -> Vec<ManagedComponentId> {
    vec![
        ManagedComponentId::YtDlp,
        ManagedComponentId::Deno,
        ManagedComponentId::Ffmpeg,
        ManagedComponentId::GalleryDl,
        ManagedComponentId::Aria2c,
    ]
}

pub(super) fn about_action_slot_width(ui: &Ui, state: &AppState) -> f32 {
    action_button_width(ui, state)
}

pub(super) fn about_check_button_width(ui: &Ui, state: &AppState) -> f32 {
    semantic_ui_metrics::standard_icon_text_button_width_for_visible_text(
        ui,
        state.ui_i18n_text_for_key("about.check_updates"),
    )
}

pub(super) fn about_install_dependencies_button_width(ui: &Ui, state: &AppState) -> f32 {
    semantic_ui_metrics::standard_icon_text_button_width_for_visible_text(
        ui,
        state.ui_i18n_text_for_key("about.install_dependencies"),
    )
}

pub(super) fn about_update_all_button_width(ui: &Ui, state: &AppState) -> f32 {
    semantic_ui_metrics::standard_icon_text_button_width_for_visible_text(
        ui,
        state.ui_i18n_text_for_key("about.update_all"),
    )
}

pub(super) fn render_about_footer_message(ui: &mut Ui, state: &AppState) {
    let text = if state.component_update_snapshot.running {
        state.localize_message(&state.component_update_snapshot.message)
    } else if !state.last_action.trim().is_empty() {
        state.localize_message(&state.last_action)
    } else {
        last_check_line(state)
    };
    ui.add(egui::Label::new(text.as_str()).truncate());
}

pub(super) fn render_check_updates_button(ui: &mut Ui, state: &mut AppState) {
    let running = state.component_update_running();
    if ui
        .add_enabled(
            !running,
            icon_text_button(
                ui,
                AppIcon::Magnify,
                state.ui_i18n_text_for_key("about.check_updates"),
            ),
        )
        .clicked()
    {
        state.check_component_updates();
    }
}

pub(super) fn render_install_dependencies_button(ui: &mut Ui, state: &mut AppState) {
    if ui
        .add_enabled(
            !state.component_update_running(),
            icon_text_button(
                ui,
                AppIcon::Download,
                state.ui_i18n_text_for_key("about.install_dependencies"),
            ),
        )
        .clicked()
    {
        state.install_missing_required_dependencies();
    }
}

pub(super) fn render_update_all_button(ui: &mut Ui, state: &mut AppState) {
    let running = state.component_update_running();
    if ui
        .add_enabled(
            !running,
            icon_text_button(
                ui,
                AppIcon::Download,
                state.ui_i18n_text_for_key("about.update_all"),
            ),
        )
        .clicked()
    {
        state.update_all_managed_components();
    }
}

/// The right-hand Item badge reports availability only, never update progress.
/// An absent optional dependency deliberately has no badge (and no status cell).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AboutAvailabilityBadge {
    Ready,
    Required,
}

fn about_availability_badge(
    tool: DependencyTool,
    installed: bool,
) -> Option<AboutAvailabilityBadge> {
    if installed {
        Some(AboutAvailabilityBadge::Ready)
    } else if matches!(
        tool,
        DependencyTool::YtDlp | DependencyTool::Deno | DependencyTool::Ffmpeg
    ) {
        Some(AboutAvailabilityBadge::Required)
    } else {
        None
    }
}

pub(super) fn about_header_status_text(state: &AppState, id: ManagedComponentId) -> Option<String> {
    // The app header has no status badge; its version uses the same title row
    // as dependency Items. Action/status messages belong to the footer.
    let tool = id.as_dependency_tool()?;
    match about_availability_badge(tool, state.dependency_tool_is_installed(tool))? {
        AboutAvailabilityBadge::Ready => Some(
            state
                .ui_i18n_text_for_key("about.availability.ready")
                .to_owned(),
        ),
        AboutAvailabilityBadge::Required => {
            Some(state.ui_i18n_text_for_key("about.required").to_owned())
        }
    }
}

pub(super) fn about_header_status_width(ui: &Ui, state: &AppState, id: ManagedComponentId) -> f32 {
    let Some(label) = about_header_status_text(state, id) else {
        return 0.0;
    };
    let label_width = super::measure::text_width(ui, &label, egui::TextStyle::Body);
    label_width + 2.0 * semantic_ui_metrics::about_component_status_badge_horizontal_padding()
}

pub(super) fn render_about_header_status(ui: &mut Ui, state: &AppState, id: ManagedComponentId) {
    let Some(label) = about_header_status_text(state, id) else {
        return;
    };
    // Reuse the app's existing Prepare status palette for both themes.
    // About badges describe availability only; an absent optional tool has none.
    let color = if id
        .as_dependency_tool()
        .is_some_and(|tool| state.dependency_tool_is_installed(tool))
    {
        super::prepare_tab::ok_color(ui)
    } else {
        super::prepare_tab::error_color(ui)
    };
    egui::Frame::NONE
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(
            semantic_ui_metrics::about_component_status_badge_horizontal_padding() as i8,
            semantic_ui_metrics::about_component_status_badge_vertical_padding() as i8,
        ))
        .show(ui, |ui| {
            ui.add(egui::Label::new(RichText::new(label).color(color)).truncate());
        });
}

pub(super) fn about_tool_has_action(state: &AppState, tool: DependencyTool) -> bool {
    if state.dependency_tool_update_is_running(tool) {
        return false;
    }
    if !state.dependency_tool_is_installed(tool) {
        return true;
    }
    state.dependency_tool_uses_managed_path(tool)
        && state
            .component_update_snapshot
            .entry(ManagedComponentId::for_dependency_tool(tool))
            .is_some_and(shows_component_action_button)
}

pub(super) fn about_app_has_action(state: &AppState) -> bool {
    shows_component_action_button(&selected_entry_for_id(state, ManagedComponentId::App))
}

pub(super) fn render_about_section_heading(ui: &mut Ui, state: &AppState, title_key: &'static str) {
    // Reuse About's existing theme-aware link/information blue in both themes.
    ui.label(
        RichText::new(state.ui_i18n_text_for_key(title_key))
            .size(semantic_ui_metrics::about_section_heading_font_size(ui))
            .color(ui.visuals().hyperlink_color)
            .strong(),
    );
}

pub(super) fn render_component_title(ui: &mut Ui, id: ManagedComponentId) {
    ui.add(egui::Label::new(RichText::new(component_display_name(id)).strong()).truncate());
}

pub(super) fn render_component_icon_cell(ui: &mut Ui, id: ManagedComponentId, icon_size: f32) {
    ui.add(component_icon_image(id, icon_size));
}

/// Static project homepages remain accessible even before the executable is installed.
pub(super) fn component_project_url(id: ManagedComponentId) -> &'static str {
    match id {
        ManagedComponentId::App => "https://github.com/kannagi0303/yt-dlp-gui-v2",
        ManagedComponentId::YtDlp => "https://github.com/yt-dlp/yt-dlp",
        ManagedComponentId::Deno => "https://github.com/denoland/deno",
        ManagedComponentId::Ffmpeg => "https://ffmpeg.org/",
        ManagedComponentId::GalleryDl => "https://codeberg.org/mikf/gallery-dl",
        ManagedComponentId::Aria2c => "https://github.com/aria2/aria2",
    }
}

/// Match Standard Mode's information links: Ctrl reveals the underline and
/// Ctrl+click opens the full URL. A regular click must not navigate.
pub(super) fn render_component_project_link(ui: &mut Ui, id: ManagedComponentId) {
    let url = component_project_url(id);
    let ctrl_down = ui.input(|input| input.modifiers.ctrl);
    let mut link = RichText::new(url)
        .text_style(egui::TextStyle::Body)
        .color(ui.visuals().hyperlink_color);
    if ctrl_down {
        link = link.underline();
    }
    // Use the same explicit interaction as Standard Mode's information links.
    // The painter's label is inert; the allocated text rect is the one click
    // target, not a second full-width row or a manually shifted rectangle.
    let label = ui.add(egui::Label::new(link).truncate());
    let response = ui.interact(
        label.rect.intersect(ui.clip_rect()),
        ui.id().with(("about-project-homepage", id)),
        Sense::click(),
    );
    if ctrl_down && response.hovered() {
        ui.output_mut(|output| output.cursor_icon = egui::CursorIcon::PointingHand);
    }
    if ctrl_down && response.clicked() {
        if let Err(error) = crate::infrastructure::open_external_url(url) {
            eprintln!("Could not open About project URL: {error}");
        }
    }
}

pub(super) fn render_component_current_version(
    ui: &mut Ui,
    state: &AppState,
    id: ManagedComponentId,
) {
    let entry = selected_entry_for_id(state, id);
    let version = if let Some(tool) = id.as_dependency_tool() {
        if !state.dependency_tool_is_installed(tool) {
            None
        } else {
            super::about_tool_version::selected_version(
                ui.ctx(),
                tool,
                state.dependency_tool_path(tool),
            )
            .and_then(|value| display_component_version(id, &value))
        }
    } else {
        entry
            .local_version
            .as_deref()
            .and_then(|version| display_component_version(id, version))
    };
    let current = version.unwrap_or_else(|| "-".to_owned());
    let latest = if let Some(tool) = id.as_dependency_tool() {
        state
            .dependency_tool_is_installed(tool)
            .then(|| {
                entry
                    .latest_version
                    .as_deref()
                    .and_then(|raw| display_component_version(id, raw))
            })
            .flatten()
    } else {
        entry
            .latest_version
            .as_deref()
            .and_then(|raw| display_component_version(id, raw))
    };
    // The update worker can describe the managed binary even when the user
    // selected an external executable. Never hide an external version arrow
    // solely because the managed copy is marked UpToDate/Installed.
    let managed_is_current = id.as_dependency_tool().is_some_and(|tool| {
        state.dependency_tool_uses_managed_path(tool)
            && matches!(
                entry.status,
                ComponentUpdateStatus::Installed | ComponentUpdateStatus::UpToDate
            )
    });
    let show_newer = latest.as_ref().is_some_and(|latest| {
        current != "-"
            && current != *latest
            && !managed_is_current
            && (id != ManagedComponentId::App || entry.app_has_newer_checked_release())
    });
    if show_newer {
        let latest = latest.expect("available when show_newer");
        let hovered = state
            .component_update_snapshot
            .entry(id)
            .is_some_and(|entry| entry.status == ComponentUpdateStatus::UpdateAvailable)
            && (id == ManagedComponentId::App && !state.component_update_running()
                || id.as_dependency_tool().is_some_and(|tool| {
                    state.dependency_tool_uses_managed_path(tool)
                        && !state.component_update_running()
                }))
            && ui.ctx().data(|data| {
                data.get_temp::<bool>(about_update_hover_id(id))
                    .unwrap_or(false)
            });
        let mut job = egui::text::LayoutJob::default();
        let text_color = ui.visuals().text_color();
        let font_id = egui::TextStyle::Body.resolve(ui.style());
        job.append(
            &format!("{current} → "),
            0.0,
            egui::TextFormat {
                font_id: font_id.clone(),
                color: text_color,
                ..Default::default()
            },
        );
        job.append(
            &latest,
            0.0,
            egui::TextFormat {
                font_id,
                color: if hovered {
                    ui.visuals().hyperlink_color
                } else {
                    text_color
                },
                ..Default::default()
            },
        );
        ui.add(egui::Label::new(job).truncate());
    } else {
        ui.add(egui::Label::new(current).truncate());
    }
}

fn about_update_hover_id(id: ManagedComponentId) -> egui::Id {
    egui::Id::new(("about-managed-tool-update-hovered", id))
}

pub(super) fn render_component_action_cell(
    ui: &mut Ui,
    state: &mut AppState,
    id: ManagedComponentId,
) {
    let entry = selected_entry_for_id(state, id);
    if let Some(tool) = id.as_dependency_tool() {
        if state.dependency_tool_update_is_running(tool) {
            return;
        }
        if !state.dependency_tool_is_installed(tool) {
            let running = state.component_update_running();
            if ui
                .add_enabled(
                    !running,
                    egui::Button::new(state.ui_i18n_text_for_key("about.install")),
                )
                .clicked()
            {
                state.install_dependency_tool(tool);
            }
            return;
        }

        if !state.dependency_tool_uses_managed_path(tool) {
            return;
        }
    }
    if shows_component_action_button(&entry) {
        render_component_action(ui, state, &entry);
    } else if id == ManagedComponentId::App {
        let status = status_text(state, &entry);
        ui.add(egui::Label::new(status).truncate());
    }
}

pub(super) fn about_display_path(stored: &str, app_root: &Path) -> String {
    if stored.trim().is_empty() {
        return String::new();
    }
    let selected = PathBuf::from(stored.trim());
    let absolute = if selected.is_absolute() {
        selected
    } else {
        app_root.join(selected)
    };
    let absolute = absolute.canonicalize().unwrap_or(absolute);
    let root = app_root
        .canonicalize()
        .unwrap_or_else(|_| app_root.to_path_buf());
    if let Ok(relative) = absolute.strip_prefix(&root) {
        format!(".\\{}", relative.display().to_string().replace('/', "\\"))
    } else {
        absolute.display().to_string()
    }
}

pub(super) fn render_tool_path_text_box(
    ui: &mut Ui,
    state: &AppState,
    tool: DependencyTool,
    size: super::xaml_layout_contracts::LayoutSize,
) {
    let saved = state.dependency_tool_path(tool);
    let missing = !state.dependency_tool_is_installed(tool);
    let mut value = if missing {
        state
            .ui_i18n_text_for_key("about.availability.missing")
            .to_owned()
    } else {
        about_display_path(
            saved,
            &crate::infrastructure::runtime_paths::application_root(),
        )
    };
    let progress = state
        .component_update_snapshot
        .entry(ManagedComponentId::for_dependency_tool(tool))
        .filter(|entry| {
            state.dependency_tool_update_is_running(tool)
                && matches!(
                    entry.status,
                    ComponentUpdateStatus::Downloading | ComponentUpdateStatus::Applying
                )
        })
        .and_then(|entry| entry.progress)
        .map(|percent| percent as f32 / 100.0);
    // Keep the selected path readable while updating an existing executable;
    // for a missing tool there is no path to show yet, so show progress text.
    if missing && state.dependency_tool_update_is_running(tool) {
        value = status_text(
            state,
            &selected_entry_for_id(state, ManagedComponentId::for_dependency_tool(tool)),
        );
    }
    AppTextBox::new(&mut value)
        .language(state.language())
        .syntax(if missing {
            AppTextBoxSyntax::Plain
        } else {
            AppTextBoxSyntax::Path
        })
        .error(missing && !state.dependency_tool_update_is_running(tool))
        .progress_background(progress)
        .desired_width(size.width)
        .desired_height(size.height)
        .min_rows(1)
        .max_rows(Some(1))
        .allow_newline(false)
        .editable(false)
        .selectable(true)
        .ui(ui);
}

pub(super) fn render_tool_browse_button(ui: &mut Ui, state: &mut AppState, tool: DependencyTool) {
    let text = state.ui_i18n_text_for_key("advance.browse");
    if ui
        .add(icon_text_button(ui, AppIcon::FolderSettings, text))
        .clicked()
    {
        choose_dependency_tool_path(state, tool);
    }
}

fn choose_dependency_tool_path(state: &mut AppState, tool: DependencyTool) {
    let current = state.dependency_tool_path(tool).trim().to_owned();
    let mut dialog = rfd::FileDialog::new()
        .add_filter(
            state.ui_i18n_text_for_key("options.filter_executable"),
            &["exe"],
        )
        .set_title(format!(
            "{} {} {}",
            state.ui_i18n_text_for_key("options.choose"),
            tool.label(),
            state.ui_i18n_text_for_key("options.executable")
        ));
    if !current.is_empty() {
        let current_path = PathBuf::from(&current);
        if let Some(parent) = current_path.parent().filter(|path| path.is_dir()) {
            dialog = dialog.set_directory(parent);
        }
    }
    if let Some(path) = dialog.set_file_name(tool.executable_name()).pick_file() {
        match tool {
            DependencyTool::YtDlp => state.set_yt_dlp_path(path.display().to_string()),
            DependencyTool::GalleryDl => state.set_gallery_dl_path(path.display().to_string()),
            DependencyTool::Ffmpeg => state.set_ffmpeg_path(path.display().to_string()),
            DependencyTool::Aria2c => state.set_aria2c_path(path.display().to_string()),
            DependencyTool::Deno => state.set_deno_path(path.display().to_string()),
        }
    }
}

fn render_component_action(ui: &mut Ui, state: &mut AppState, entry: &ComponentUpdateEntry) {
    let running = state.component_update_running();
    if entry.id == ManagedComponentId::App && entry.status == ComponentUpdateStatus::PendingRestart
    {
        if ui
            .add_enabled(
                !running,
                egui::Button::new(state.ui_i18n_text_for_key("about.restart")),
            )
            .clicked()
        {
            match state.restart_to_apply_app_update() {
                Ok(()) => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                Err(error) => state.last_action = error,
            }
        }
        return;
    }

    if should_show_update_button(entry) {
        let response = ui.add_enabled(!running, egui::Button::new(action_label(state, entry)));
        let hovered = response.hovered() && !running;
        let hover_id = about_update_hover_id(entry.id);
        let old_hovered = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(hover_id).unwrap_or(false));
        if old_hovered != hovered {
            ui.ctx()
                .data_mut(|data| data.insert_temp(hover_id, hovered));
            ui.ctx().request_repaint();
        }
        if response.clicked() {
            state.update_component(entry.id);
        }
    } else {
        ui.label(RichText::new(status_text(state, entry)).color(subtle_text_color(ui)));
    }
}

fn selected_entry_for_id(state: &AppState, id: ManagedComponentId) -> ComponentUpdateEntry {
    state
        .component_update_snapshot
        .entry(id)
        .cloned()
        .unwrap_or_else(|| ComponentUpdateEntry::new(id))
}

pub(super) fn component_display_name(id: ManagedComponentId) -> &'static str {
    match id {
        ManagedComponentId::App => "yt-dlp-gui",
        ManagedComponentId::YtDlp => "yt-dlp",
        ManagedComponentId::GalleryDl => "gallery-dl",
        ManagedComponentId::Deno => "Deno",
        ManagedComponentId::Ffmpeg => "FFmpeg",
        ManagedComponentId::Aria2c => "Aria2",
    }
}

fn action_button_width(ui: &Ui, state: &AppState) -> f32 {
    [
        state.ui_i18n_text_for_key("about.update"),
        state.ui_i18n_text_for_key("about.install"),
        state.ui_i18n_text_for_key("about.restart"),
    ]
    .into_iter()
    .map(|text| semantic_ui_metrics::standard_action_button_width_for_visible_text(ui, text))
    .fold(0.0, f32::max)
}

fn display_component_version(id: ManagedComponentId, version: &str) -> Option<String> {
    let version = version.trim();
    if version.is_empty() || version == "-" {
        return None;
    }

    if let Some(date) = dotted_date_from_yyyymmdd_tail(version) {
        return Some(date);
    }
    if let Some(date) = dotted_date_from_iso8601_prefix(version) {
        return Some(date);
    }

    let normalized = version
        .strip_prefix('v')
        .or_else(|| version.strip_prefix("release-"))
        .unwrap_or(version)
        .trim();

    if id == ManagedComponentId::Ffmpeg && normalized.eq_ignore_ascii_case("latest") {
        return None;
    }

    Some(normalized.to_owned())
}

fn dotted_date_from_iso8601_prefix(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    if bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
    {
        return Some(format!(
            "{}.{}.{}",
            &value[0..4],
            &value[5..7],
            &value[8..10]
        ));
    }
    None
}

fn dotted_date_from_yyyymmdd_tail(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() < 8 {
        return None;
    }
    for start in (0..=bytes.len() - 8).rev() {
        let end = start + 8;
        if !bytes[start..end].iter().all(u8::is_ascii_digit) {
            continue;
        }
        if start > 0 && bytes[start - 1].is_ascii_digit() {
            continue;
        }
        if end < bytes.len() && bytes[end].is_ascii_digit() {
            continue;
        }
        let date = &value[start..end];
        return Some(format!("{}.{}.{}", &date[0..4], &date[4..6], &date[6..8]));
    }
    None
}

fn should_show_latest_version(entry: &ComponentUpdateEntry) -> bool {
    matches!(
        entry.status,
        ComponentUpdateStatus::UpdateAvailable
            | ComponentUpdateStatus::Missing
            | ComponentUpdateStatus::PendingRestart
            | ComponentUpdateStatus::Downloading
            | ComponentUpdateStatus::Staged
            | ComponentUpdateStatus::Applying
            | ComponentUpdateStatus::Failed
    ) || (entry.id == ManagedComponentId::App && entry.local_version.is_none())
}

fn should_show_update_button(entry: &ComponentUpdateEntry) -> bool {
    matches!(
        entry.status,
        ComponentUpdateStatus::UpdateAvailable | ComponentUpdateStatus::Missing
    ) && matches!(
        entry.ownership,
        ComponentOwnership::ManagedPortable | ComponentOwnership::Missing
    )
}

fn shows_component_action_button(entry: &ComponentUpdateEntry) -> bool {
    (entry.id == ManagedComponentId::App && entry.status == ComponentUpdateStatus::PendingRestart)
        || should_show_update_button(entry)
}

fn action_label<'a>(state: &'a AppState, entry: &ComponentUpdateEntry) -> &'a str {
    if entry.status == ComponentUpdateStatus::Missing {
        state.ui_i18n_text_for_key("about.install")
    } else {
        state.ui_i18n_text_for_key("about.update")
    }
}

pub(super) fn last_check_line(state: &AppState) -> String {
    if state.component_update_snapshot.running {
        return state.ui_i18n_text_for_key("about.running").to_owned();
    }
    if let Some(time) = state.component_update_snapshot.checked_at_unix {
        format!(
            "{} {}",
            state.ui_i18n_text_for_key("about.last_check"),
            format_last_check_value(state, time)
        )
    } else {
        state.ui_i18n_text_for_key("about.never_checked").to_owned()
    }
}

fn format_last_check_value(state: &AppState, checked_at_unix: u64) -> String {
    let now = now_unix_seconds().unwrap_or(checked_at_unix);
    let age = now.saturating_sub(checked_at_unix);
    let minute = 60;
    let hour = 60 * minute;
    let day = 24 * hour;

    if age < hour {
        let minutes = (age / minute).max(1).to_string();
        return state.ui_i18n_text_with_replacements(
            "about.relative.minutes",
            &[("{count}", minutes.as_str())],
        );
    }
    if age < day {
        let hours = (age / hour).max(1);
        return if hours == 1 {
            state.ui_i18n_text_for_key("about.relative.hour").to_owned()
        } else {
            let hours = hours.to_string();
            state.ui_i18n_text_with_replacements(
                "about.relative.hours",
                &[("{count}", hours.as_str())],
            )
        };
    }
    if age < 7 * day {
        let days = (age / day).max(1);
        return if days == 1 {
            state.ui_i18n_text_for_key("about.relative.day").to_owned()
        } else {
            let days = days.to_string();
            state.ui_i18n_text_with_replacements(
                "about.relative.days",
                &[("{count}", days.as_str())],
            )
        };
    }

    format_unix_date_utc(checked_at_unix)
}

fn now_unix_seconds() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

fn format_unix_date_utc(unix_seconds: u64) -> String {
    let days = (unix_seconds / 86_400) as i64;
    let (year, month, day) = civil_from_days_utc(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days_utc(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    // Howard Hinnant civil calendar conversion. Kept local to avoid adding a
    // date/time dependency for one compact relative timestamp in the About page.
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m as u32, d as u32)
}

fn subtle_text_color(ui: &Ui) -> Color32 {
    ui.visuals().text_color().linear_multiply(0.68)
}

fn status_text(state: &AppState, entry: &ComponentUpdateEntry) -> String {
    match entry.status {
        ComponentUpdateStatus::Unknown => state
            .ui_i18n_text_for_key("about.status.unknown")
            .to_owned(),
        ComponentUpdateStatus::Checking => state
            .ui_i18n_text_for_key("about.status.checking")
            .to_owned(),
        ComponentUpdateStatus::UpToDate => state
            .ui_i18n_text_for_key("about.status.up_to_date")
            .to_owned(),
        ComponentUpdateStatus::UpdateAvailable => state
            .ui_i18n_text_for_key("about.status.update_available")
            .to_owned(),
        ComponentUpdateStatus::Missing => state
            .ui_i18n_text_for_key("about.status.missing")
            .to_owned(),
        ComponentUpdateStatus::Downloading => entry
            .progress
            .map(|percent| {
                let percent = percent.to_string();
                state.ui_i18n_text_with_replacements(
                    "about.status.downloading_percent",
                    &[("{percent}", percent.as_str())],
                )
            })
            .unwrap_or_else(|| {
                state
                    .ui_i18n_text_for_key("about.status.downloading")
                    .to_owned()
            }),
        ComponentUpdateStatus::Staged => {
            state.ui_i18n_text_for_key("about.status.staged").to_owned()
        }
        ComponentUpdateStatus::PendingRestart => state
            .ui_i18n_text_for_key("about.status.pending_restart")
            .to_owned(),
        ComponentUpdateStatus::Applying if !entry.message.trim().is_empty() => {
            state.localize_message(&entry.message)
        }
        ComponentUpdateStatus::Applying => state
            .ui_i18n_text_for_key("about.status.applying")
            .to_owned(),
        ComponentUpdateStatus::Installed => state
            .ui_i18n_text_for_key("about.status.installed")
            .to_owned(),
        ComponentUpdateStatus::Skipped => state
            .ui_i18n_text_for_key("about.status.skipped")
            .to_owned(),
        ComponentUpdateStatus::Failed => {
            state.ui_i18n_text_for_key("about.status.failed").to_owned()
        }
    }
}
