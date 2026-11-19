//! System tray icon (StatusNotifierItem, shown by KDE Plasma and most
//! Linux desktops).
//!
//! The menu is rebuilt from the shared state each time it is shown. Menu
//! callbacks never talk to the earbuds directly: they send an [`Action`] to
//! the connection worker.

use ksni::menu::{CheckmarkItem, RadioGroup, RadioItem, StandardItem, SubMenu};
use std::sync::OnceLock;

use ksni::{Icon, MenuItem, ToolTip};
use nothing_link::Status;
use nothing_proto::{AncMode, BassEnhance, Capabilities, EqPreset, model};
use tokio::sync::mpsc::UnboundedSender;

use crate::state::{Action, Link, Shared};
use crate::{UiBridge, logo};

pub struct NothingTray {
    pub shared: Shared,
    pub actions: UnboundedSender<Action>,
    pub ui: UiBridge,
}

type Item = MenuItem<NothingTray>;

impl NothingTray {
    fn send(&self, action: Action) {
        let _ = self.actions.send(action);
    }
}

impl ksni::Tray for NothingTray {
    fn id(&self) -> String {
        "nothing-ear".into()
    }

    fn title(&self) -> String {
        "Nothing earbuds".into()
    }

    /// The app logo, with grey dots while the earbuds are not connected.
    fn icon_pixmap(&self) -> Vec<Icon> {
        let connected = matches!(self.shared.lock().link, Link::Connected);
        tray_icons(connected).to_vec()
    }

    fn tool_tip(&self) -> ToolTip {
        let state = self.shared.snapshot();
        let mut lines = Vec::new();
        if let Some(battery) = state.battery_summary() {
            lines.push(battery);
        }
        if let Some(anc) = state.status.as_ref().and_then(|status| status.anc) {
            lines.push(anc.label().to_owned());
        }
        ToolTip {
            title: state.link_label(),
            description: lines.join("\n"),
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
        }
    }

    /// Left click opens the settings window.
    fn activate(&mut self, _x: i32, _y: i32) {
        self.ui.open();
    }

    fn menu(&self) -> Vec<Item> {
        let state = self.shared.snapshot();
        let mut items = vec![disabled_label(state.link_label(), "")];
        if let Some(battery) = state.battery_summary() {
            items.push(disabled_label(battery, "battery"));
        }
        items.push(MenuItem::Separator);

        match (&state.link, &state.status) {
            (Link::Connected, Some(status)) => {
                items.extend(quick_settings(status));
                items.push(MenuItem::Separator);
            }
            _ => items.push(action_item("Retry now", "view-refresh", |tray| {
                tray.send(Action::Refresh)
            })),
        }

        items.push(action_item("All settings...", "configure", |tray| tray.ui.open()));
        items.push(action_item("Quit", "application-exit", |tray| tray.ui.quit()));
        items
    }
}

/// Quick settings shown in the menu while connected.
fn quick_settings(status: &Status) -> Vec<Item> {
    let caps = model::from_bluetooth_name(&status.name).capabilities;
    let mut items = Vec::new();

    if let Some(mode) = status.anc {
        items.push(anc_menu(mode, caps));
    }
    if let Some(preset) = status.eq_preset {
        items.push(eq_menu(preset));
    }
    if let Some(bass) = status.bass {
        items.push(bass_menu(bass));
    }
    if let Some(enabled) = status.low_latency {
        items.push(checkbox("Low latency", enabled, move |tray| {
            tray.send(Action::SetLowLatency(!enabled))
        }));
    }
    if let Some(enabled) = status.in_ear_detection {
        items.push(checkbox("In-ear detection", enabled, move |tray| {
            tray.send(Action::SetInEarDetection(!enabled))
        }));
    }
    items
}

fn anc_menu(current: AncMode, caps: Capabilities) -> Item {
    let modes: Vec<AncMode> = AncMode::ALL
        .into_iter()
        .filter(|mode| *mode != AncMode::Adaptive || caps.adaptive_anc)
        .collect();
    let labels = modes.iter().map(|mode| mode.label().to_owned()).collect();
    let selected = modes.iter().position(|mode| *mode == current);

    submenu(
        format!("Noise control: {}", current.label()),
        "audio-volume-muted",
        vec![radio_group(labels, selected, move |tray, index| {
            if let Some(&mode) = modes.get(index) {
                tray.send(Action::SetAnc(mode));
            }
        })],
    )
}

fn eq_menu(current: EqPreset) -> Item {
    let presets = EqPreset::SELECTABLE;
    let labels = presets.iter().map(|preset| preset.label()).collect();
    let selected = presets.iter().position(|preset| *preset == current);

    submenu(
        format!("Equalizer: {}", current.label()),
        "view-media-equalizer",
        vec![radio_group(labels, selected, move |tray, index| {
            if let Some(&preset) = presets.get(index) {
                tray.send(Action::SetEqPreset(preset));
            }
        })],
    )
}

fn bass_menu(bass: BassEnhance) -> Item {
    let toggle = checkbox("Enabled", bass.enabled, move |tray| {
        tray.send(Action::SetBass(BassEnhance {
            enabled: !bass.enabled,
            ..bass
        }))
    });
    let level_labels = (1..=BassEnhance::MAX_LEVEL)
        .map(|level| format!("Level {level}"))
        .collect();
    let selected = Some(usize::from(bass.level.saturating_sub(1)));
    let levels = radio_group(level_labels, selected, |tray, index| {
        let level = index as u8 + 1;
        tray.send(Action::SetBass(BassEnhance { enabled: true, level }))
    });

    submenu(
        "Bass enhance".into(),
        "",
        vec![toggle, MenuItem::Separator, levels],
    )
}

// --- Small builders around ksni's menu types ---------------------------------

fn disabled_label(label: String, icon: &str) -> Item {
    StandardItem {
        label,
        enabled: false,
        icon_name: icon.into(),
        ..Default::default()
    }
    .into()
}

fn action_item(label: &str, icon: &str, on_click: impl Fn(&mut NothingTray) + Send + Sync + 'static) -> Item {
    StandardItem {
        label: label.into(),
        icon_name: icon.into(),
        activate: Box::new(on_click),
        ..Default::default()
    }
    .into()
}

fn checkbox(label: &str, checked: bool, on_click: impl Fn(&mut NothingTray) + Send + Sync + 'static) -> Item {
    CheckmarkItem {
        label: label.into(),
        checked,
        activate: Box::new(on_click),
        ..Default::default()
    }
    .into()
}

fn submenu(label: String, icon: &str, items: Vec<Item>) -> Item {
    SubMenu {
        label,
        icon_name: icon.into(),
        submenu: items,
        ..Default::default()
    }
    .into()
}

/// Radio buttons. `selected` is `None` when the current value is not in the
/// list (for example a custom EQ preset): then nothing is checked.
fn radio_group(
    labels: Vec<String>,
    selected: Option<usize>,
    on_select: impl Fn(&mut NothingTray, usize) + Send + Sync + 'static,
) -> Item {
    RadioGroup {
        selected: selected.unwrap_or(usize::MAX),
        select: Box::new(on_select),
        options: labels
            .into_iter()
            .map(|label| RadioItem {
                label,
                ..Default::default()
            })
            .collect(),
    }
    .into()
}

/// The logo in a few sizes, so the panel can pick the sharpest one. Drawn
/// once per state and kept: the panel asks for it on every update.
fn tray_icons(connected: bool) -> &'static [Icon] {
    static CONNECTED: OnceLock<Vec<Icon>> = OnceLock::new();
    static NOT_CONNECTED: OnceLock<Vec<Icon>> = OnceLock::new();
    let cache = if connected { &CONNECTED } else { &NOT_CONNECTED };
    cache.get_or_init(|| {
        [22, 32, 48, 64]
            .into_iter()
            .map(|size| Icon {
                width: size as i32,
                height: size as i32,
                // The tray wants ARGB, egui and the logo use RGBA.
                data: logo::rgba(size, connected)
                    .chunks_exact(4)
                    .flat_map(|p| [p[3], p[0], p[1], p[2]])
                    .collect(),
            })
            .collect()
    })
}
