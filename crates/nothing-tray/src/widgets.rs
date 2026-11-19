//! Small reusable widgets for the settings window.
//!
//! They keep `window.rs` focused on *what* is shown rather than *how*.

use eframe::egui::{self, CornerRadius, RichText, Sense, Stroke, StrokeKind, Vec2};

use crate::theme::{CARD_RADIUS, WIDGET_RADIUS, caps, caption, dot_text};

/// A rounded box grouping related settings, with an optional title.
pub fn card<R>(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let visuals = ui.visuals();
    egui::Frame::new()
        .fill(visuals.faint_bg_color)
        .stroke(visuals.widgets.noninteractive.bg_stroke)
        .corner_radius(CornerRadius::same(CARD_RADIUS))
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                ui.label(caption(title));
                ui.add_space(2.0);
            }
            add_contents(ui)
        })
        .inner
}

/// Row with a label (and optional hint below it) on the left and a switch on
/// the right. Returns the new value when the user flips the switch.
pub fn switch_row(ui: &mut egui::Ui, label: &str, hint: &str, value: bool) -> Option<bool> {
    let mut on = value;
    ui.horizontal(|ui| {
        // Inside a row, text does not wrap on its own: cap its width so a
        // long hint wraps instead of pushing the switch out of the window.
        let text_width = ui.available_width() - SWITCH_SIZE.x - ui.spacing().item_spacing.x * 2.0;
        ui.vertical(|ui| {
            ui.set_max_width(text_width);
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.label(caps(label));
            if !hint.is_empty() {
                ui.label(RichText::new(hint).small().weak());
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            switch(ui, &mut on);
        });
    });
    (on != value).then_some(on)
}

const SWITCH_SIZE: Vec2 = Vec2::new(36.0, 20.0);

/// On/off switch: a filled track when on, an empty one when off.
pub fn switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(SWITCH_SIZE, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Switch, ui.is_enabled(), *on, ""));

    if ui.is_rect_visible(rect) {
        let position = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.visuals();
        let radius = rect.height() / 2.0;
        let (track, knob_color) = if *on {
            (visuals.selection.bg_fill, visuals.selection.stroke.color)
        } else {
            (visuals.widgets.inactive.bg_fill, visuals.weak_text_color())
        };
        let border = if response.hovered() {
            visuals.widgets.hovered.bg_stroke
        } else {
            Stroke::NONE
        };
        ui.painter().rect(rect, radius, track, border, StrokeKind::Inside);
        let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), position);
        let knob = egui::pos2(knob_x, rect.center().y);
        ui.painter().circle(knob, radius - 4.0, knob_color, Stroke::NONE);
    }
    response
}

const SEGMENT_HEIGHT: f32 = 28.0;

/// Buttons where exactly one is selected, filling the available width.
/// When the labels do not fit on one line, the buttons are split into
/// balanced rows. Returns the option the user clicked, if it is not the
/// current one.
pub fn segmented<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    current: Option<T>,
    options: &[(T, &str)],
) -> Option<T> {
    if options.is_empty() {
        return None;
    }
    let spacing = ui.spacing().item_spacing.x;
    let available = ui.available_width();
    let per_row = buttons_per_row(ui, options, available, spacing);
    let width = (available - spacing * (per_row as f32 - 1.0)) / per_row as f32;

    let mut picked = None;
    for row in options.chunks(per_row) {
        ui.horizontal(|ui| {
            for &(value, label) in row {
                let selected = current == Some(value);
                let button = egui::Button::selectable(selected, caps(label))
                    .frame_when_inactive(true)
                    .truncate() // never grow wider than the computed width
                    .min_size(Vec2::new(width, SEGMENT_HEIGHT))
                    .corner_radius(CornerRadius::same(WIDGET_RADIUS));
                if ui.add_sized([width, SEGMENT_HEIGHT], button).clicked() && !selected {
                    picked = Some(value);
                }
            }
        });
    }
    picked
}

/// How many buttons to put on each row so that the widest label fits, with
/// rows as even as possible (5 options that need 2 rows give 3 + 2).
fn buttons_per_row<T>(ui: &egui::Ui, options: &[(T, &str)], available: f32, spacing: f32) -> usize {
    // The button frame also draws a stroke on each side: leave room for it,
    // or the longest label gets truncated.
    let padding = ui.spacing().button_padding.x * 2.0 + 4.0;
    let widest = options
        .iter()
        .map(|(_, label)| {
            let galley = egui::WidgetText::from(caps(label)).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Button,
            );
            galley.size().x + padding
        })
        .fold(0.0, f32::max);

    let fitting = ((available + spacing) / (widest + spacing)).floor().max(1.0) as usize;
    let count = options.len();
    let rows = count.div_ceil(fitting.min(count));
    count.div_ceil(rows)
}

/// Battery level of one part: a caption above the level in big dot-matrix
/// figures, greyed out when unknown.
pub fn battery_chip(ui: &mut egui::Ui, label: &str, level: Option<nothing_proto::BatteryLevel>) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let label = match level {
            Some(level) if level.charging => format!("{label} · charging"),
            _ => label.to_string(),
        };
        ui.label(caption(label));
        match level {
            Some(level) => ui.label(dot_text(format!("{}%", level.percent), BATTERY_SIZE)),
            None => ui.label(dot_text("--", BATTERY_SIZE).weak()),
        };
    });
}

const BATTERY_SIZE: f32 = 26.0;
