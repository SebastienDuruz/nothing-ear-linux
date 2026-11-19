//! Background task that owns the connection to the earbuds.
//!
//! It runs forever:
//! 1. wait until Nothing earbuds are connected over Bluetooth;
//! 2. open the control channel and read every setting;
//! 3. apply actions from the UI, follow notifications from the earbuds and
//!    refresh the battery from time to time;
//! 4. when the connection drops, go back to step 1.

use std::time::Duration;

use nothing_link::{ConnectOptions, Earbuds, Event, LinkError};
use nothing_proto::Battery;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::state::{Action, FitTest, Link, Shared};

/// Delay between two searches while no earbuds are connected.
const RETRY_DELAY: Duration = Duration::from_secs(5);
/// Battery refresh interval, on top of the battery notifications.
const BATTERY_REFRESH: Duration = Duration::from_secs(120);
/// Lets the earbuds release the channel before reconnecting.
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// Called after every state change, to refresh the tray icon and the window.
pub type Notify = Box<dyn Fn() + Send + Sync>;

pub async fn run(shared: Shared, mut actions: UnboundedReceiver<Action>, notify: Notify) {
    loop {
        set_link(&shared, &notify, Link::Connecting);
        let earbuds = match Earbuds::connect(&ConnectOptions::default()).await {
            Ok(earbuds) => earbuds,
            Err(error) => {
                let link = match error {
                    LinkError::NotFound => Link::Searching,
                    other => Link::Error(other.to_string()),
                };
                set_link(&shared, &notify, link);
                if !wait_before_retry(&mut actions).await {
                    return; // the application is closing
                }
                continue;
            }
        };

        let status = earbuds.status().await;
        {
            let mut state = shared.lock();
            state.status = Some(status);
            state.link = Link::Connected;
            state.last_error = None;
        }
        notify();

        let app_closing = !run_session(&earbuds, &shared, &mut actions, &notify).await;
        if app_closing {
            return;
        }

        reset_after_disconnect(&shared);
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// Waits [`RETRY_DELAY`], or less if the user asks to retry. Actions sent
/// while disconnected are dropped. Returns `false` if the app is closing.
async fn wait_before_retry(actions: &mut UnboundedReceiver<Action>) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(RETRY_DELAY) => {}
        action = actions.recv() => {
            if action.is_none() {
                return false;
            }
        }
    }
    while actions.try_recv().is_ok() {}
    true
}

/// Handles one connection until it drops. Returns `false` if the app is closing.
async fn run_session(
    earbuds: &Earbuds,
    shared: &Shared,
    actions: &mut UnboundedReceiver<Action>,
    notify: &Notify,
) -> bool {
    let mut events = earbuds.subscribe();
    let mut battery_timer = tokio::time::interval(BATTERY_REFRESH);
    battery_timer.tick().await; // the first tick is immediate; the status was just read

    loop {
        tokio::select! {
            action = actions.recv() => {
                let Some(action) = action else { return false };
                if let Err(error) = apply(earbuds, shared, notify, action).await {
                    log::warn!("{error}");
                    shared.lock().last_error = Some(error.to_string());
                    notify();
                    if error.is_connection_lost() {
                        return true;
                    }
                }
            }

            event = events.recv() => match event {
                Ok(Event::Battery(battery)) => {
                    store_battery(shared, battery);
                    notify();
                }
                Ok(Event::Anc(mode)) => {
                    shared.update_status(|status| status.anc = Some(mode));
                    notify();
                }
                Ok(Event::Disconnected) | Err(RecvError::Closed) => return true,
                Ok(Event::EarFit(_) | Event::Unsolicited(_)) | Err(RecvError::Lagged(_)) => {}
            },

            _ = battery_timer.tick() => match earbuds.battery().await {
                Ok(battery) => {
                    store_battery(shared, battery);
                    notify();
                }
                Err(error) if error.is_connection_lost() => return true,
                Err(error) => log::warn!("{error}"),
            },
        }
    }
}

/// Applies an action, then reads the setting back so the UI shows what the
/// earbuds actually report.
async fn apply(earbuds: &Earbuds, shared: &Shared, notify: &Notify, action: Action) -> Result<(), LinkError> {
    match action {
        Action::Refresh => {
            let status = earbuds.status().await;
            shared.lock().status = Some(status);
        }
        Action::SetAnc(mode) => {
            earbuds.set_anc(mode).await?;
            let mode = earbuds.anc().await?;
            shared.update_status(|s| s.anc = Some(mode));
        }
        Action::SetEqPreset(preset) => {
            earbuds.set_eq_preset(preset).await?;
            let preset = earbuds.eq_preset().await?;
            shared.update_status(|s| s.eq_preset = Some(preset));
        }
        Action::SetCustomEq(eq) => {
            earbuds.set_custom_eq(eq).await?;
            let eq = earbuds.custom_eq().await?;
            let preset = earbuds.eq_preset().await?; // the preset may switch to "custom"
            shared.update_status(|s| {
                s.custom_eq = Some(eq);
                s.eq_preset = Some(preset);
            });
        }
        Action::SetBass(bass) => {
            earbuds.set_bass(bass).await?;
            let bass = earbuds.bass().await?;
            shared.update_status(|s| s.bass = Some(bass));
        }
        Action::SetLowLatency(enabled) => {
            earbuds.set_low_latency(enabled).await?;
            let enabled = earbuds.low_latency().await?;
            shared.update_status(|s| s.low_latency = Some(enabled));
        }
        Action::SetInEarDetection(enabled) => {
            earbuds.set_in_ear_detection(enabled).await?;
            let enabled = earbuds.in_ear_detection().await?;
            shared.update_status(|s| s.in_ear_detection = Some(enabled));
        }
        Action::SetGesture(gesture) => {
            earbuds.set_gesture(gesture).await?;
            let gestures = earbuds.gestures().await?;
            shared.update_status(|s| s.gestures = Some(gestures));
        }
        Action::Ring(target, ringing) => {
            earbuds.ring(target, ringing).await?;
            shared.lock().ringing = ringing;
        }
        Action::EarFitTest => {
            shared.lock().fit_test = FitTest::Running;
            notify();
            let result = earbuds.ear_fit_test().await;
            shared.lock().fit_test = match &result {
                Ok(fit) => FitTest::Done(*fit),
                Err(error) => FitTest::Failed(error.to_string()),
            };
            result?;
        }
    }
    shared.lock().last_error = None;
    notify();
    Ok(())
}

/// Stores a battery reading. The case is only reported while its lid is
/// open, so the last known case level is kept otherwise.
fn store_battery(shared: &Shared, battery: Battery) {
    shared.update_status(|status| {
        let previous_case = status.battery.and_then(|old| old.case);
        status.battery = Some(Battery {
            case: battery.case.or(previous_case),
            ..battery
        });
    });
}

fn reset_after_disconnect(shared: &Shared) {
    let mut state = shared.lock();
    state.ringing = false;
    if matches!(state.fit_test, FitTest::Running) {
        state.fit_test = FitTest::Failed("connection lost".into());
    }
}

/// Updates the connection state, and forgets the settings when not connected.
fn set_link(shared: &Shared, notify: &Notify, link: Link) {
    let changed = {
        let mut state = shared.lock();
        let changed = state.link != link;
        if link != Link::Connected {
            state.status = None;
        }
        state.link = link;
        changed
    };
    if changed {
        notify();
    }
}
