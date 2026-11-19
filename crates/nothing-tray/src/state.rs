//! State shared by the connection worker, the tray icon and the window.

use std::sync::{Arc, Mutex, MutexGuard};

use nothing_link::Status;
use nothing_proto::{AncMode, BassEnhance, BatteryLevel, CustomEq, EarFit, EqPreset, Gesture, RingTarget};

/// Request sent by the tray or the window to the connection worker.
#[derive(Debug, Clone)]
pub enum Action {
    /// Read every setting again (or retry the connection right away).
    Refresh,
    SetAnc(AncMode),
    SetEqPreset(EqPreset),
    SetCustomEq(CustomEq),
    SetBass(BassEnhance),
    SetLowLatency(bool),
    SetInEarDetection(bool),
    SetGesture(Gesture),
    Ring(RingTarget, bool),
    EarFitTest,
}

/// State of the connection to the earbuds.
#[derive(Debug, Clone, PartialEq)]
pub enum Link {
    /// No Nothing earbuds connected over Bluetooth.
    Searching,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Debug, Clone, Default)]
pub enum FitTest {
    #[default]
    Idle,
    Running,
    Done(EarFit),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub link: Link,
    /// Last known settings; `None` while not connected.
    pub status: Option<Status>,
    /// Error from the last action, shown in the window.
    pub last_error: Option<String>,
    pub fit_test: FitTest,
    pub ringing: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            link: Link::Searching,
            status: None,
            last_error: None,
            fit_test: FitTest::Idle,
            ringing: false,
        }
    }
}

impl AppState {
    /// Short battery summary such as "L 85% · R 85% · Case 60%".
    pub fn battery_summary(&self) -> Option<String> {
        let battery = self.status.as_ref()?.battery?;
        let part = |label: &str, level: Option<BatteryLevel>| {
            level.map(|l| format!("{label} {}%{}", l.percent, if l.charging { " ⚡" } else { "" }))
        };
        let parts: Vec<String> = [
            part("L", battery.left),
            part("R", battery.right),
            part("Case", battery.case),
        ]
        .into_iter()
        .flatten()
        .collect();
        (!parts.is_empty()).then(|| parts.join(" · "))
    }

    /// One-line description of the connection.
    pub fn link_label(&self) -> String {
        match &self.link {
            Link::Searching => "Earbuds not connected".into(),
            Link::Connecting => "Connecting...".into(),
            Link::Connected => match &self.status {
                Some(status) => status.name.clone(),
                None => "Connected".into(),
            },
            Link::Error(error) => format!("Error: {error}"),
        }
    }
}

/// Cheap-to-clone handle on the shared state.
#[derive(Clone, Default)]
pub struct Shared(Arc<Mutex<AppState>>);

impl Shared {
    pub fn lock(&self) -> MutexGuard<'_, AppState> {
        // A panic in another thread must not freeze the UI.
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Copy of the state, so the UI does not hold the lock while drawing.
    pub fn snapshot(&self) -> AppState {
        self.lock().clone()
    }

    /// Changes the last known settings, if connected.
    pub fn update_status(&self, change: impl FnOnce(&mut Status)) {
        if let Some(status) = self.lock().status.as_mut() {
            change(status);
        }
    }
}
