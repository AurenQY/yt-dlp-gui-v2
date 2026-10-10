//! Gallery-only filter chain. No dependency on yt-dlp picker state or renderer.
use super::gallery_range_text;
use super::xaml_ui_nodes::{TemplateNode, auto, block, cols, fill, fixed_px, gap, rows};
use super::{semantic_ui_metrics as metrics, xaml_template_renderer as template};
use crate::app::state::{AppState, GalleryRangeSection};
use crate::domain::GalleryDownloadRange;
use eframe::egui::{self, Rect, Ui};
use egui_taffy::Tui;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Choice {
    Category(Option<GalleryRangeSection>),
    Preset(u32),
    Custom,
    Unlimited,
}

#[derive(Clone, Copy)]
enum Node {
    Title(&'static str),
    Choice(Choice),
    Input,
    Spacer,
}

struct FilterStage {
    title: &'static str,
    choices: Vec<(Choice, String)>,
    width: f32,
    title_width: f32,
}

impl FilterStage {
    fn resolve(ui: &Ui, title: &'static str, choices: Vec<(Choice, String)>) -> Self {
        let width = metrics::format_picker_filter_stage_node_width_for_visible_values(
            ui,
            choices.iter().map(|(_, label)| label.as_str()),
            metrics::format_picker_filter_button_padding().x,
        );
        Self {
            title,
            choices,
            width,
            title_width: super::measure::text_width(ui, title, egui::TextStyle::Body),
        }
    }

    fn tree(&self, title_height: f32, node_height: f32) -> TemplateNode<Node> {
        let mut children = vec![
            fixed_px(
                title_height,
                cols([
                    fill(block(Node::Spacer)),
                    fixed_px(self.title_width, block(Node::Title(self.title))),
                    fill(block(Node::Spacer)),
                ]),
            ),
            gap(metrics::format_picker_filter_stage_title_to_nodes_vertical_spacing()),
        ];
        for (choice, _) in &self.choices {
            children.push(fixed_px(
                node_height,
                cols([
                    fill(block(Node::Spacer)),
                    fixed_px(self.width, block(Node::Choice(*choice))),
                    fill(block(Node::Spacer)),
                ]),
            ));
            children.push(gap(metrics::format_picker_filter_node_vertical_spacing()));
        }
        rows(children)
    }
}

pub(super) fn render(slot: template::TemplateBlockSlot, tui: &mut Tui, state: &mut AppState) {
    let language = state.language();
    let section = state.gallery_range_picker.section;
    let ui = tui.egui_ui();
    let title_height = ui.text_style_height(&egui::TextStyle::Body);
    let node_height = metrics::format_picker_filter_node_height_from_current_control_metrics(ui);
    let left = FilterStage::resolve(
        ui,
        crate::i18n::text(state.language(), "gallery.range.method"),
        vec![
            (
                Choice::Category(Some(GalleryRangeSection::Time)),
                crate::i18n::text(state.language(), "gallery.range.time").to_owned(),
            ),
            (
                Choice::Category(Some(GalleryRangeSection::Posts)),
                crate::i18n::text(state.language(), "gallery.range.posts").to_owned(),
            ),
            (
                Choice::Category(None),
                crate::i18n::text(state.language(), "item.all").to_owned(),
            ),
        ],
    );
    let choices = match section {
        Some(GalleryRangeSection::Time) => [1, 7, 30]
            .into_iter()
            .map(|days| {
                (
                    Choice::Preset(days),
                    gallery_range_text::summary(
                        &GalleryDownloadRange::Recent {
                            days,
                            since: None,
                            until: None,
                        },
                        state.language(),
                    ),
                )
            })
            .chain([(
                Choice::Custom,
                crate::i18n::text(state.language(), "gallery.range.custom").to_owned(),
            )])
            .collect(),
        Some(GalleryRangeSection::Posts) => [10, 50, 100]
            .into_iter()
            .map(|count| {
                (
                    Choice::Preset(count),
                    gallery_range_text::summary(
                        &GalleryDownloadRange::Posts { count },
                        state.language(),
                    ),
                )
            })
            .chain([(
                Choice::Custom,
                crate::i18n::text(state.language(), "gallery.range.custom").to_owned(),
            )])
            .collect(),
        None => vec![(
            Choice::Unlimited,
            crate::i18n::text(state.language(), "item.all").to_owned(),
        )],
    };
    let right = FilterStage::resolve(
        ui,
        crate::i18n::text(state.language(), "advance.download_range"),
        choices,
    );
    let mut stages = vec![
        fill(left.tree(title_height, node_height)),
        fill(right.tree(title_height, node_height)),
    ];
    if state.gallery_range_picker.custom_selected {
        let input_title = if section == Some(GalleryRangeSection::Time) {
            crate::i18n::text(state.language(), "gallery.range.date")
        } else {
            crate::i18n::text(state.language(), "gallery.range.count")
        };
        let input_width = super::measure::text_width(ui, "YYYY-MM-DD", egui::TextStyle::Body)
            + ui.spacing().button_padding.x * 2.0;
        let title_width = super::measure::text_width(ui, input_title, egui::TextStyle::Body);
        stages.push(fill(rows([
            fixed_px(
                title_height,
                cols([
                    fill(block(Node::Spacer)),
                    fixed_px(title_width, block(Node::Title(input_title))),
                    fill(block(Node::Spacer)),
                ]),
            ),
            gap(metrics::format_picker_filter_stage_title_to_nodes_vertical_spacing()),
            fixed_px(
                node_height,
                cols([
                    fill(block(Node::Spacer)),
                    fixed_px(input_width.max(title_width), block(Node::Input)),
                    fill(block(Node::Spacer)),
                ]),
            ),
        ])));
    }
    let tree = rows([auto(cols(stages))]);
    // This painter only decorates actual selected response rects. It never positions widgets.
    let painter = ui.painter().clone();
    let line_color = ui.visuals().widgets.active.bg_fill;
    let mut left_rect = None;
    let mut right_rect = None;
    let mut input_rect = None;
    template::show_template_tui_block(slot, tui, |tui| {
        template::show_auto_height_template(tree, tui, &mut |slot, node, tui| {
            template::show_template_ui_block(slot, tui, |ui| match node {
                Node::Title(text) => {
                    ui.add(egui::Label::new(egui::RichText::new(text).strong()).selectable(false));
                }
                Node::Choice(choice) => {
                    let stage = if matches!(choice, Choice::Category(_)) {
                        &left
                    } else {
                        &right
                    };
                    let label = &stage
                        .choices
                        .iter()
                        .find(|(value, _)| *value == choice)
                        .unwrap()
                        .1;
                    let selected = is_selected(state, choice, section);
                    let enabled = match choice {
                        Choice::Category(Some(GalleryRangeSection::Posts)) => {
                            state.gallery_post_range_available()
                        }
                        Choice::Preset(_) | Choice::Custom
                            if section == Some(GalleryRangeSection::Posts) =>
                        {
                            state.gallery_post_range_available()
                        }
                        _ => true,
                    };
                    ui.spacing_mut().button_padding =
                        metrics::format_picker_filter_button_padding();
                    let response = ui
                        .centered_and_justified(|ui| {
                            ui.add_enabled(enabled, egui::Button::new(label).selected(selected))
                        })
                        .inner;
                    if selected {
                        if matches!(choice, Choice::Category(_)) {
                            left_rect = Some(response.rect);
                        } else {
                            right_rect = Some(response.rect);
                        }
                    }
                    if response.clicked() {
                        select_choice(state, choice, section);
                    }
                }
                Node::Spacer => {}
                Node::Input => {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut state.gallery_range_picker.custom)
                            .hint_text(if section == Some(GalleryRangeSection::Time) {
                                "YYYY-MM-DD"
                            } else {
                                crate::i18n::text(language, "gallery.range.positive")
                            })
                            .desired_width(f32::INFINITY),
                    );
                    input_rect = Some(response.rect);
                    if response.changed() {
                        let value = state.gallery_range_picker.custom.trim().to_owned();
                        let result = if section == Some(GalleryRangeSection::Time) {
                            crate::domain::parse_gallery_date(&value)
                                .map(|_| GalleryDownloadRange::Since { date: value })
                                .map_err(|_| {
                                    crate::i18n::text(
                                        state.language(),
                                        "gallery.range.invalid_date",
                                    )
                                    .to_owned()
                                })
                        } else {
                            value
                                .parse::<u32>()
                                .ok()
                                .filter(|n| *n > 0)
                                .map(|count| GalleryDownloadRange::Posts { count })
                                .ok_or_else(|| {
                                    crate::i18n::text(state.language(), "gallery.range.invalid")
                                        .to_owned()
                                })
                        };
                        match result {
                            Ok(range) => {
                                state.gallery_range_picker.selected = Some(range);
                                state.gallery_range_picker.error = None;
                            }
                            Err(error) => {
                                state.gallery_range_picker.selected = None;
                                state.gallery_range_picker.error = Some(error);
                            }
                        }
                    }
                }
            });
        });
    });
    if state.gallery_range_picker.section == section {
        if let (Some(left), Some(right)) = (left_rect, right_rect) {
            draw_connection(
                &painter,
                left,
                right,
                egui::Stroke::new(
                    metrics::format_picker_filter_connection_stroke_width(),
                    line_color,
                ),
            );
        }
        if let (Some(right), Some(input)) = (right_rect, input_rect) {
            draw_connection(
                &painter,
                right,
                input,
                egui::Stroke::new(
                    metrics::format_picker_filter_connection_stroke_width(),
                    line_color,
                ),
            );
        }
    }
}

fn is_selected(state: &AppState, choice: Choice, section: Option<GalleryRangeSection>) -> bool {
    let draft = &state.gallery_range_picker;
    match choice {
        Choice::Category(value) => value == section,
        Choice::Custom => draft.custom_selected,
        Choice::Unlimited => matches!(draft.selected, Some(GalleryDownloadRange::All)),
        Choice::Preset(n) => {
            !draft.custom_selected
                && match &draft.selected {
                    Some(GalleryDownloadRange::Posts { count }) => *count == n,
                    Some(GalleryDownloadRange::Recent { days, .. }) => *days == n,
                    _ => false,
                }
        }
    }
}

fn select_choice(state: &mut AppState, choice: Choice, section: Option<GalleryRangeSection>) {
    let draft = &mut state.gallery_range_picker;
    match choice {
        Choice::Category(value) if value != section => {
            draft.section = value;
            draft.custom_selected = false;
            draft.custom.clear();
            draft.error = None;
            draft.selected = value.is_none().then_some(GalleryDownloadRange::All);
        }
        Choice::Preset(n) => {
            draft.selected = Some(if section == Some(GalleryRangeSection::Time) {
                GalleryDownloadRange::Recent {
                    days: n,
                    since: None,
                    until: None,
                }
            } else {
                GalleryDownloadRange::Posts { count: n }
            });
            draft.custom_selected = false;
            draft.error = None;
        }
        Choice::Custom if !draft.custom_selected => {
            draft.custom_selected = true;
            draft.custom.clear();
            draft.selected = None;
            draft.error = None;
        }
        Choice::Unlimited => draft.selected = Some(GalleryDownloadRange::All),
        _ => {}
    }
}

// Matches the yt-dlp filter's decorative path, kept local so its implementation is untouched.
fn draw_connection(painter: &egui::Painter, left: Rect, right: Rect, stroke: egui::Stroke) {
    let start = egui::pos2(left.right(), left.center().y);
    let end = egui::pos2(right.left(), right.center().y);
    let mid_x = (start.x + end.x) * 0.5;
    let dy = end.y - start.y;
    if dy.abs() < 1.0 {
        painter.line_segment([start, end], stroke);
        return;
    }
    let direction = dy.signum();
    let radius = (dy.abs() * 0.25).clamp(8.0, 14.0);
    let a = egui::pos2(mid_x - radius, start.y);
    let b = egui::pos2(mid_x, start.y + direction * radius);
    let c = egui::pos2(mid_x, end.y - direction * radius);
    let d = egui::pos2(mid_x + radius, end.y);
    painter.line_segment([start, a], stroke);
    painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
        [
            a,
            egui::pos2(mid_x - radius * 0.45, start.y),
            egui::pos2(mid_x, start.y + direction * radius * 0.55),
            b,
        ],
        false,
        egui::Color32::TRANSPARENT,
        stroke,
    ));
    painter.line_segment([b, c], stroke);
    painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
        [
            c,
            egui::pos2(mid_x, end.y - direction * radius * 0.55),
            egui::pos2(mid_x + radius * 0.45, end.y),
            d,
        ],
        false,
        egui::Color32::TRANSPARENT,
        stroke,
    ));
    painter.line_segment([d, end], stroke);
}
