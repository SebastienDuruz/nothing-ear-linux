//! Settings window (egui).
//!
//! Layout:
//! - header: earbuds name and battery levels;
//! - three tabs: Sound, Gestures, Device;
//! - each tab is a column of cards, one card per topic.
//!
//! The window never talks to the earbuds directly: it draws a snapshot of
//! the shared state and sends an [`Action`] to the connection worker when
//! the user changes something. The worker reads the setting back, updates
//! the shared state and asks for a repaint.

use std::sync::mpsc::Receiver;
use std::time::Duration;

use eframe::egui::{self, RichText};
use nothing_link::Status;
use nothing_proto::{
    AncMode, BassEnhance, Capabilities, CustomEq, EqPreset, FitResult, Gesture, GestureAction, GestureKind,
    RingTarget, Side, model, settings,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::UiRequest;
use crate::logo;
use crate::state::{Action, AppState, FitTest, Link, Shared};
use crate::theme::{self, ERROR_COLOR, caps, caption, dot_text};
use crate::widgets::{battery_chip, card, segmented, switch_row};

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Sound,
    Gestures,
    Device,
}

/// The three choices shown first for noise control. The strength of noise
/// cancellation is picked in a second row.
#[derive(Clone, Copy, PartialEq)]
enum NoiseChoice {
    Off,
    Transparency,
    Cancellation,
}

/// Equalizer choices: the presets, plus the custom equalizer.
#[derive(Clone, Copy, PartialEq)]
enum EqChoice {
    Preset(EqPreset),
    Custom,
}

pub struct SettingsWindow<'a> {
    shared: Shared,
    actions: UnboundedSender<Action>,
    /// Requests from the tray icon ("open", "quit") while the window is open.
    requests: &'a Receiver<UiRequest>,
    /// Set to true when the whole app must exit after the window closes.
    quit: &'a mut bool,

    // --- UI-only state, never sent to the earbuds as is ---
    tab: Tab,
    /// The custom equalizer panel is open (before anything is applied).
    editing_custom_eq: bool,
    /// Custom EQ being edited; sent when "Apply" is clicked, and kept until
    /// the earbuds report the same values.
    eq_draft: Option<CustomEq>,
    /// Side shown in the Gestures tab.
    gesture_side: Side,
    /// The user confirmed that the earbuds are not in their ears.
    ring_confirmed: bool,
}

impl<'a> SettingsWindow<'a> {
    pub fn new(
        shared: Shared,
        actions: UnboundedSender<Action>,
        requests: &'a Receiver<UiRequest>,
        quit: &'a mut bool,
    ) -> Self {
        Self {
            shared,
            actions,
            requests,
            quit,
            tab: Tab::Sound,
            editing_custom_eq: false,
            eq_draft: None,
            gesture_side: Side::Left,
            ring_confirmed: false,
        }
    }

    fn send(&self, action: Action) {
        let _ = self.actions.send(action);
    }
}

impl eframe::App for SettingsWindow<'_> {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(request) = self.requests.try_recv() {
            match request {
                UiRequest::Open => ctx.send_viewport_cmd(egui::ViewportCommand::Focus),
                UiRequest::Quit => {
                    *self.quit = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The worker requests repaints on changes; this is only a safety net.
        ui.ctx().request_repaint_after(Duration::from_secs(1));
        let state = self.shared.snapshot();

        egui::Frame::central_panel(ui.style())
            .inner_margin(16.0)
            .show(ui, |ui| {
                // Fill the whole window with the background colour, even
                // when the content is short.
                ui.set_min_size(ui.available_size());
                match (&state.link, &state.status) {
                    (Link::Connected, Some(status)) => self.draw_connected(ui, &state, status),
                    _ => self.draw_not_connected(ui, &state),
                }
            });
    }
}

// --- Top level -----------------------------------------------------------------

impl SettingsWindow<'_> {
    fn draw_not_connected(&self, ui: &mut egui::Ui, state: &AppState) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 3.0);
            logo::paint(ui, 96.0);
            ui.add_space(8.0);
            ui.label(dot_text(state.link_label(), 22.0));
            match state.link {
                Link::Connecting => {
                    ui.spinner();
                }
                _ => {
                    ui.label(
                        RichText::new(
                            "Connect your earbuds over Bluetooth.\nThey are detected automatically.",
                        )
                        .weak(),
                    );
                    ui.add_space(8.0);
                    if ui.button(caps("Retry now")).clicked() {
                        self.send(Action::Refresh);
                    }
                }
            }
        });
    }

    fn draw_connected(&mut self, ui: &mut egui::Ui, state: &AppState, status: &Status) {
        self.draw_header(ui, status);
        self.draw_error_banner(ui, state);

        let tabs = [
            (Tab::Sound, "Sound"),
            (Tab::Gestures, "Gestures"),
            (Tab::Device, "Device"),
        ];
        if let Some(tab) = segmented(ui, Some(self.tab), &tabs) {
            self.tab = tab;
        }
        ui.add_space(4.0);

        let caps = model::from_bluetooth_name(&status.name).capabilities;
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| match self.tab {
                Tab::Sound => self.draw_sound_tab(ui, status, caps),
                Tab::Gestures => self.draw_gestures_tab(ui, status),
                Tab::Device => self.draw_device_tab(ui, state, status, caps),
            });
    }

    fn draw_header(&self, ui: &mut egui::Ui, status: &Status) {
        ui.label(dot_text(status.name.as_str(), 30.0));
        ui.label(caption(status.model_code.as_str()));
        ui.add_space(6.0);
        if let Some(battery) = status.battery {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 32.0;
                battery_chip(ui, "Left", battery.left);
                battery_chip(ui, "Right", battery.right);
                battery_chip(ui, "Case", battery.case);
            });
        }
        ui.add_space(12.0);
    }

    fn draw_error_banner(&self, ui: &mut egui::Ui, state: &AppState) {
        let Some(error) = &state.last_error else { return };
        egui::Frame::new()
            .stroke(egui::Stroke::new(1.0, ERROR_COLOR))
            .corner_radius(theme::WIDGET_RADIUS)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Leave room for the close button: a long message wraps
                    // instead of widening the window.
                    let text_width = ui.available_width() - 32.0;
                    ui.vertical(|ui| {
                        ui.set_max_width(text_width);
                        ui.label(RichText::new(format!("⚠ {error}")).small().color(ERROR_COLOR));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✕").clicked() {
                            self.shared.lock().last_error = None;
                        }
                    });
                });
            });
        ui.add_space(4.0);
    }
}

// --- Sound tab -----------------------------------------------------------------

impl SettingsWindow<'_> {
    fn draw_sound_tab(&mut self, ui: &mut egui::Ui, status: &Status, caps: Capabilities) {
        if let Some(mode) = status.anc {
            card(ui, "Noise control", |ui| self.draw_noise_control(ui, mode, caps));
        }
        if status.eq_preset.is_some() {
            card(ui, "Equalizer", |ui| self.draw_equalizer(ui, status));
        }
        if let Some(bass) = status.bass {
            card(ui, "", |ui| self.draw_bass(ui, bass));
        }
        if let Some(enabled) = status.low_latency {
            card(ui, "", |ui| {
                let hint = "Reduces audio delay for games and videos";
                if let Some(enabled) = switch_row(ui, "Low latency", hint, enabled) {
                    self.send(Action::SetLowLatency(enabled));
                }
            });
        }
    }

    fn draw_noise_control(&self, ui: &mut egui::Ui, mode: AncMode, caps: Capabilities) {
        let current = match mode {
            AncMode::Off => NoiseChoice::Off,
            AncMode::Transparency => NoiseChoice::Transparency,
            _ => NoiseChoice::Cancellation,
        };
        let choices = [
            (NoiseChoice::Off, "Off"),
            (NoiseChoice::Transparency, "Transparency"),
            (NoiseChoice::Cancellation, "Noise cancelling"),
        ];
        match segmented(ui, Some(current), &choices) {
            Some(NoiseChoice::Off) => self.send(Action::SetAnc(AncMode::Off)),
            Some(NoiseChoice::Transparency) => self.send(Action::SetAnc(AncMode::Transparency)),
            Some(NoiseChoice::Cancellation) => {
                let default = if caps.adaptive_anc {
                    AncMode::Adaptive
                } else {
                    AncMode::High
                };
                self.send(Action::SetAnc(default));
            }
            None => {}
        }

        if current == NoiseChoice::Cancellation {
            ui.label(caption("Strength"));
            let mut strengths = vec![
                (AncMode::High, "High"),
                (AncMode::Mid, "Mid"),
                (AncMode::Low, "Low"),
            ];
            if caps.adaptive_anc {
                strengths.insert(0, (AncMode::Adaptive, "Adaptive"));
            }
            if let Some(strength) = segmented(ui, Some(mode), &strengths) {
                self.send(Action::SetAnc(strength));
            }
        }
    }

    fn draw_equalizer(&mut self, ui: &mut egui::Ui, status: &Status) {
        // An unknown preset code is most likely the custom equalizer too.
        let current = match status.eq_preset {
            _ if self.editing_custom_eq => EqChoice::Custom,
            Some(EqPreset::Custom | EqPreset::Other(_)) => EqChoice::Custom,
            Some(preset) => EqChoice::Preset(preset),
            None => EqChoice::Custom,
        };
        let mut choices: Vec<(EqChoice, String)> = EqPreset::SELECTABLE
            .into_iter()
            .map(|preset| (EqChoice::Preset(preset), preset.label()))
            .collect();
        if status.custom_eq.is_some() {
            choices.push((EqChoice::Custom, "Custom".into()));
        }
        let choices: Vec<(EqChoice, &str)> = choices.iter().map(|(c, label)| (*c, label.as_str())).collect();

        match segmented(ui, Some(current), &choices) {
            Some(EqChoice::Preset(preset)) => {
                self.editing_custom_eq = false;
                self.eq_draft = None;
                self.send(Action::SetEqPreset(preset));
            }
            Some(EqChoice::Custom) => {
                // Selecting "Custom" plays the bands already stored on the earbuds.
                self.editing_custom_eq = true;
                self.send(Action::SetEqPreset(EqPreset::Custom));
            }
            None => {}
        }

        if current == EqChoice::Custom {
            if let Some(saved) = status.custom_eq {
                self.draw_custom_eq(ui, saved);
            }
        }
    }

    fn draw_custom_eq(&mut self, ui: &mut egui::Ui, saved: CustomEq) {
        ui.add_space(4.0);
        let draft = self.eq_draft.get_or_insert(saved);
        let range = settings::CUSTOM_EQ_MIN_DB..=settings::CUSTOM_EQ_MAX_DB;
        egui::Grid::new("custom_eq")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                for (label, gain) in [
                    ("Bass", &mut draft.bass_db),
                    ("Mid", &mut draft.mid_db),
                    ("Treble", &mut draft.treble_db),
                ] {
                    ui.label(caps(label));
                    ui.add(egui::Slider::new(gain, range.clone()).step_by(0.5).suffix(" dB"));
                    ui.end_row();
                }
            });

        let draft = *draft;
        let modified = draft != saved;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(modified, egui::Button::new(caps("Apply")))
                .clicked()
            {
                // The draft is kept until the earbuds report it back: clearing
                // it now would show the old values again (the worker has not
                // read the new ones yet), and a second "Apply" would send them.
                self.send(Action::SetCustomEq(draft));
            }
            if ui
                .add_enabled(modified, egui::Button::new(caps("Cancel")))
                .clicked()
            {
                self.eq_draft = None;
            }
            if ui.button(caps("Reset")).clicked() {
                self.eq_draft = Some(CustomEq::default());
            }
        });

        // The earbuds now hold the draft values: follow them again.
        if self.eq_draft == Some(saved) {
            self.eq_draft = None;
        }
    }

    fn draw_bass(&self, ui: &mut egui::Ui, bass: BassEnhance) {
        if let Some(enabled) = switch_row(ui, "Bass enhance", "Adds depth to low frequencies", bass.enabled) {
            self.send(Action::SetBass(BassEnhance { enabled, ..bass }));
        }
        if !bass.enabled {
            return;
        }

        ui.label(caption("Level"));
        let levels = [(1, "1"), (2, "2"), (3, "3"), (4, "4"), (5, "5")];
        if let Some(level) = segmented(ui, Some(bass.level), &levels) {
            self.send(Action::SetBass(BassEnhance { enabled: true, level }));
        }
    }
}

// --- Gestures tab --------------------------------------------------------------

impl SettingsWindow<'_> {
    fn draw_gestures_tab(&mut self, ui: &mut egui::Ui, status: &Status) {
        if let Some(gestures) = &status.gestures {
            card(ui, "Pinch controls", |ui| {
                let sides = [(Side::Left, "Left earbud"), (Side::Right, "Right earbud")];
                if let Some(side) = segmented(ui, Some(self.gesture_side), &sides) {
                    self.gesture_side = side;
                }
                ui.add_space(4.0);
                self.draw_gesture_rows(ui, gestures);
            });
        }
        if let Some(enabled) = status.in_ear_detection {
            card(ui, "", |ui| {
                let hint = "Pauses music when you take an earbud out";
                if let Some(enabled) = switch_row(ui, "In-ear detection", hint, enabled) {
                    self.send(Action::SetInEarDetection(enabled));
                }
            });
        }
    }

    fn draw_gesture_rows(&self, ui: &mut egui::Ui, gestures: &[Gesture]) {
        let side = self.gesture_side;
        egui::Grid::new("gestures")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                for kind in GestureKind::ALL {
                    let current = gestures
                        .iter()
                        .find(|g| g.side == side && g.kind == kind)
                        .map(|g| g.action);
                    ui.label(caps(kind.label()));
                    if let Some(action) = gesture_picker(ui, side, kind, current) {
                        self.send(Action::SetGesture(Gesture { side, kind, action }));
                    }
                    ui.end_row();
                }
            });
    }
}

/// Drop-down list of gesture actions. Returns the new action when the user
/// picks a different one.
fn gesture_picker(
    ui: &mut egui::Ui,
    side: Side,
    kind: GestureKind,
    current: Option<GestureAction>,
) -> Option<GestureAction> {
    let mut picked = None;
    let current_label = current.map(|action| action.label()).unwrap_or_else(|| "-".into());
    let current_label = caps(current_label);
    egui::ComboBox::from_id_salt(("gesture", side, kind))
        .selected_text(current_label)
        .width(ui.available_width())
        // A long action name is cut with "…" rather than widening the window.
        .truncate()
        .show_ui(ui, |ui| {
            for action in GestureAction::SELECTABLE {
                let selected = current == Some(action);
                if ui.selectable_label(selected, caps(action.label())).clicked() && !selected {
                    picked = Some(action);
                }
            }
        });
    picked
}

// --- Device tab ----------------------------------------------------------------

impl SettingsWindow<'_> {
    fn draw_device_tab(&mut self, ui: &mut egui::Ui, state: &AppState, status: &Status, caps: Capabilities) {
        if caps.ring {
            card(ui, "Find my earbuds", |ui| self.draw_ring(ui, state.ringing));
        }
        if caps.ear_fit_test {
            card(ui, "Ear tip fit test", |ui| {
                self.draw_fit_test(ui, &state.fit_test)
            });
        }
        card(ui, "About", |ui| self.draw_about(ui, status));
    }

    fn draw_ring(&mut self, ui: &mut egui::Ui, ringing: bool) {
        ui.label(
            RichText::new("Plays a loud sound. Take the earbuds out of your ears first.")
                .small()
                .weak(),
        );
        ui.checkbox(&mut self.ring_confirmed, "The earbuds are not in my ears");
        ui.horizontal(|ui| {
            ui.add_enabled_ui(self.ring_confirmed, |ui| {
                for (label, target) in [
                    ("Left", RingTarget::Left),
                    ("Right", RingTarget::Right),
                    ("Both", RingTarget::Both),
                ] {
                    if ui.button(caps(label)).clicked() {
                        self.send(Action::Ring(target, true));
                    }
                }
            });
            if ringing && ui.button(caps("Stop")).clicked() {
                self.send(Action::Ring(RingTarget::Both, false));
            }
        });
    }

    fn draw_fit_test(&self, ui: &mut egui::Ui, fit_test: &FitTest) {
        ui.label(
            RichText::new("Wear both earbuds. A short sound checks how well the tips seal.")
                .small()
                .weak(),
        );
        let running = matches!(fit_test, FitTest::Running);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!running, egui::Button::new(caps("Start test")))
                .clicked()
            {
                self.send(Action::EarFitTest);
            }
            if running {
                ui.spinner();
                ui.label(caps("Testing..."));
            }
        });

        // The result goes below the button, where it has the whole width.
        match fit_test {
            FitTest::Idle | FitTest::Running => {}
            FitTest::Done(fit) => {
                let describe = |result: FitResult| match result {
                    FitResult::Good => "Good fit",
                    FitResult::Poor => "Try another tip size",
                };
                egui::Grid::new("fit_test")
                    .num_columns(2)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        for (label, result) in [("Left", fit.left), ("Right", fit.right)] {
                            ui.label(caption(label));
                            ui.label(caps(describe(result)));
                            ui.end_row();
                        }
                    });
            }
            FitTest::Failed(error) => {
                ui.colored_label(ERROR_COLOR, error);
            }
        }
    }

    fn draw_about(&self, ui: &mut egui::Ui, status: &Status) {
        egui::Grid::new("about")
            .num_columns(2)
            .spacing([24.0, 6.0])
            .show(ui, |ui| {
                for (label, value) in [
                    ("Model", Some(format!("{} ({})", status.model, status.model_code))),
                    ("Firmware", status.firmware.clone()),
                    ("Serial number", status.serial.clone()),
                    ("Bluetooth address", Some(status.address.clone())),
                ] {
                    ui.label(caption(label));
                    ui.label(value.unwrap_or_else(|| "-".into()));
                    ui.end_row();
                }
            });
        if ui.button(caps("Refresh")).clicked() {
            self.send(Action::Refresh);
        }
    }
}
