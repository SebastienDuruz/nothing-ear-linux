//! Demo mode: `NOTHING_EAR_DEMO=1 nothing-ear`.
//!
//! Opens the window with sample data and no Bluetooth connection, so the
//! interface can be worked on without earbuds (and without taking the
//! control channel away from another running instance). Changes made in
//! the window are applied to the sample data only.

use nothing_link::Status;
use nothing_proto::{
    AncMode, BassEnhance, Battery, BatteryLevel, CustomEq, EarFit, EqPreset, FitResult, Gesture,
    GestureAction, GestureKind, Side,
};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::state::{Action, FitTest, Link, Shared};
use crate::worker::Notify;

/// Environment variable that turns demo mode on.
pub const ENV_VAR: &str = "NOTHING_EAR_DEMO";

pub fn is_enabled() -> bool {
    std::env::var_os(ENV_VAR).is_some()
}

/// Stands in for the connection worker: applies actions to the sample data.
pub async fn run(shared: Shared, mut actions: UnboundedReceiver<Action>, notify: Notify) {
    {
        let mut state = shared.lock();
        state.link = Link::Connected;
        state.status = Some(sample_status());
    }
    notify();

    while let Some(action) = actions.recv().await {
        apply(&shared, action);
        notify();
    }
}

fn apply(shared: &Shared, action: Action) {
    match action {
        Action::Refresh => {}
        Action::SetAnc(mode) => shared.update_status(|s| s.anc = Some(mode)),
        Action::SetEqPreset(preset) => shared.update_status(|s| s.eq_preset = Some(preset)),
        Action::SetCustomEq(eq) => shared.update_status(|s| {
            s.custom_eq = Some(eq);
            s.eq_preset = Some(EqPreset::Custom);
        }),
        Action::SetBass(bass) => shared.update_status(|s| s.bass = Some(bass)),
        Action::SetLowLatency(enabled) => shared.update_status(|s| s.low_latency = Some(enabled)),
        Action::SetInEarDetection(enabled) => shared.update_status(|s| s.in_ear_detection = Some(enabled)),
        Action::SetGesture(new) => shared.update_status(|s| {
            for gesture in s.gestures.iter_mut().flatten() {
                if gesture.side == new.side && gesture.kind == new.kind {
                    gesture.action = new.action;
                }
            }
        }),
        Action::Ring(_, ringing) => shared.lock().ringing = ringing,
        Action::EarFitTest => {
            shared.lock().fit_test = FitTest::Done(EarFit {
                left: FitResult::Good,
                right: FitResult::Poor,
            })
        }
    }
}

/// Values read from a real pair of Nothing Ear (B171).
fn sample_status() -> Status {
    let level = |percent| {
        Some(BatteryLevel {
            percent,
            charging: false,
        })
    };
    let gesture = |side, kind, action| Gesture { side, kind, action };
    Status {
        name: "Nothing Ear".into(),
        address: "00:11:22:33:44:55".into(),
        model: "Nothing Ear".into(),
        model_code: "B171".into(),
        firmware: Some("1.0.1.54".into()),
        serial: Some("DEMO000000000000".into()),
        battery: Some(Battery {
            left: level(80),
            right: level(85),
            case: level(60),
        }),
        anc: Some(AncMode::Transparency),
        eq_preset: Some(EqPreset::Balanced),
        custom_eq: Some(CustomEq::default()),
        bass: Some(BassEnhance {
            enabled: true,
            level: 3,
        }),
        low_latency: Some(false),
        in_ear_detection: Some(true),
        gestures: Some(vec![
            gesture(Side::Left, GestureKind::DoublePinch, GestureAction::NextTrack),
            gesture(Side::Left, GestureKind::TriplePinch, GestureAction::PreviousTrack),
            gesture(
                Side::Left,
                GestureKind::PinchAndHold,
                GestureAction::VoiceAssistant,
            ),
            gesture(
                Side::Left,
                GestureKind::DoublePinchAndHold,
                GestureAction::NoAction,
            ),
            gesture(Side::Right, GestureKind::DoublePinch, GestureAction::NextTrack),
            gesture(
                Side::Right,
                GestureKind::TriplePinch,
                GestureAction::PreviousTrack,
            ),
            gesture(
                Side::Right,
                GestureKind::PinchAndHold,
                GestureAction::NoiseControlAncTransparency,
            ),
            gesture(
                Side::Right,
                GestureKind::DoublePinchAndHold,
                GestureAction::NoAction,
            ),
        ]),
    }
}
