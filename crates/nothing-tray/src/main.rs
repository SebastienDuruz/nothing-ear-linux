//! `nothing-ear`: system tray icon and settings window for Nothing earbuds.
//!
//! Options:
//! - `--hidden`: start in the tray without opening the window (for autostart).
//!
//! Environment:
//! - `NOTHING_EAR_DEMO=1`: window only, with sample data and no Bluetooth (see [`demo`]).
//!
//! Threads:
//! - a tokio runtime runs the connection worker ([`worker`]) and the tray
//!   icon ([`tray`]);
//! - the main thread runs the window ([`window`]), because GUI toolkits must
//!   live on the main thread. It opens the window on request and can open it
//!   again after it was closed.
//!
//! The three parts share an [`state::AppState`]. The tray and the window
//! send [`state::Action`]s to the worker, which is the only part talking to
//! the earbuds.

mod demo;
mod logo;
mod state;
mod theme;
mod tray;
mod widgets;
mod window;
mod worker;

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use ksni::TrayMethods;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use crate::state::{Action, Shared};

/// Request sent to the main thread.
pub enum UiRequest {
    Open,
    Quit,
}

/// Sends requests to the main thread and wakes the window up if it is open.
#[derive(Clone)]
pub struct UiBridge {
    requests: Sender<UiRequest>,
    /// Context of the open window, used to request a repaint.
    window: Arc<Mutex<Option<egui::Context>>>,
}

impl UiBridge {
    pub fn open(&self) {
        self.send(UiRequest::Open);
    }

    pub fn quit(&self) {
        self.send(UiRequest::Quit);
    }

    pub fn repaint(&self) {
        if let Some(ctx) = self.window_context().as_ref() {
            ctx.request_repaint();
        }
    }

    fn send(&self, request: UiRequest) {
        let _ = self.requests.send(request);
        self.repaint(); // so an open window handles the request right away
    }

    fn window_context(&self) -> std::sync::MutexGuard<'_, Option<egui::Context>> {
        self.window
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .format_timestamp(None)
        .init();
    let start_hidden = std::env::args().any(|arg| arg == "--hidden");
    let demo = demo::is_enabled();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("failed to start the tokio runtime");

    let shared = Shared::default();
    let (actions_tx, actions_rx) = unbounded_channel();
    let (requests_tx, requests_rx) = channel();
    let ui = UiBridge {
        requests: requests_tx,
        window: Arc::default(),
    };

    // Tray icon. Without a tray host (some desktops), or in demo mode, the
    // window is the only way in, so closing it quits the app.
    let tray = tray::NothingTray {
        shared: shared.clone(),
        actions: actions_tx.clone(),
        ui: ui.clone(),
    };
    let tray_handle = match demo {
        true => None,
        false => match runtime.block_on(tray.spawn()) {
            Ok(handle) => Some(handle),
            Err(error) => {
                log::warn!("no tray icon ({error}); closing the window will quit the app");
                None
            }
        },
    };

    // Called by the worker after every change: refresh the tray and the window.
    let notify: worker::Notify = {
        let tray_handle = tray_handle.clone();
        let ui = ui.clone();
        let runtime = runtime.handle().clone();
        Box::new(move || {
            if let Some(handle) = tray_handle.clone() {
                runtime.spawn(async move { handle.update(|_| {}).await });
            }
            ui.repaint();
        })
    };
    if demo {
        runtime.spawn(demo::run(shared.clone(), actions_rx, notify));
    } else {
        runtime.spawn(worker::run(shared.clone(), actions_rx, notify));
    }

    let has_tray = tray_handle.is_some();
    if !start_hidden || !has_tray {
        ui.open();
    }
    run_main_thread(&shared, &actions_tx, &ui, &requests_rx, has_tray);
    runtime.shutdown_timeout(Duration::from_secs(1));
}

/// Waits for requests and opens the window as many times as asked.
fn run_main_thread(
    shared: &Shared,
    actions: &UnboundedSender<Action>,
    ui: &UiBridge,
    requests: &Receiver<UiRequest>,
    has_tray: bool,
) {
    loop {
        match requests.recv() {
            Ok(UiRequest::Open) => {}
            Ok(UiRequest::Quit) | Err(_) => return,
        }
        // Several "open" requests may have piled up: one window is enough.
        while let Ok(UiRequest::Open) = requests.try_recv() {}

        let mut quit = false;
        let result = eframe::run_native(
            "nothing-ear",
            window_options(),
            Box::new(|creation| {
                *ui.window_context() = Some(creation.egui_ctx.clone());
                theme::install(&creation.egui_ctx);
                Ok(Box::new(window::SettingsWindow::new(
                    shared.clone(),
                    actions.clone(),
                    requests,
                    &mut quit,
                )))
            }),
        );
        *ui.window_context() = None;

        if let Err(error) = result {
            log::error!("window error: {error}");
        }
        if quit || !has_tray {
            return;
        }
    }
}

/// Fixed window size: the layout is designed for this width (every row of
/// buttons fits on one line). Longer content scrolls vertically.
const WINDOW_SIZE: [f32; 2] = [560.0, 660.0];
const WINDOW_ICON_SIZE: u32 = 256;

fn window_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Nothing earbuds")
            .with_app_id("nothing-ear")
            .with_inner_size(WINDOW_SIZE)
            // Same minimum and maximum too, for compositors that ignore
            // the "not resizable" hint.
            .with_min_inner_size(WINDOW_SIZE)
            .with_max_inner_size(WINDOW_SIZE)
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(egui::IconData {
                rgba: logo::rgba(WINDOW_ICON_SIZE, true),
                width: WINDOW_ICON_SIZE,
                height: WINDOW_ICON_SIZE,
            }),
        // Return from `run_native` when the window closes, so it can be reopened.
        run_and_return: true,
        ..Default::default()
    }
}
