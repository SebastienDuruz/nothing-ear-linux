//! Low-level exchange of frames over the RFCOMM socket.
//!
//! A background task reads the socket. Each incoming frame is either:
//! - the reply to a pending request (same operation number and command id),
//!   handed back to the caller of [`Client::request`]; or
//! - a spontaneous message, broadcast as an [`Event`].

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bluer::rfcomm::Stream;
use bluer::rfcomm::stream::{OwnedReadHalf, OwnedWriteHalf};
use nothing_proto::{AncMode, Battery, EarFit, Frame, FrameDecoder, command};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::LinkError;

/// Default time to wait for a reply.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);

/// How long ENOTCONN errors are treated as temporary after connecting.
///
/// `bluer` returns from `connect` as soon as the non-blocking socket becomes
/// writable, which can happen before the kernel has finished opening the
/// RFCOMM channel. The first reads and writes may then fail with ENOTCONN.
/// During this window we retry instead of treating the link as lost.
const SETTLE_WINDOW: Duration = Duration::from_secs(3);
const SETTLE_RETRY_DELAY: Duration = Duration::from_millis(50);

/// Message sent by the earbuds without being asked.
#[derive(Debug, Clone)]
pub enum Event {
    Battery(Battery),
    Anc(AncMode),
    EarFit(EarFit),
    /// Frame that matches no request and that we do not decode.
    Unsolicited(Frame),
    /// The RFCOMM connection was closed.
    Disconnected,
}

/// A request waiting for its reply, keyed by operation number.
struct PendingRequest {
    command_id: u8,
    reply: oneshot::Sender<Frame>,
}

type PendingRequests = Arc<Mutex<HashMap<u8, PendingRequest>>>;

pub struct Client {
    writer: tokio::sync::Mutex<OwnedWriteHalf>,
    pending: PendingRequests,
    next_operation: AtomicU8,
    events: broadcast::Sender<Event>,
    reader_task: JoinHandle<()>,
    connected_at: Instant,
}

impl Client {
    pub fn new(stream: Stream) -> Self {
        let (read_half, write_half) = stream.into_split();
        let pending = PendingRequests::default();
        let (events, _) = broadcast::channel(64);
        let connected_at = Instant::now();

        let reader_task = tokio::spawn(read_loop(
            read_half,
            pending.clone(),
            events.clone(),
            connected_at,
        ));

        Self {
            writer: tokio::sync::Mutex::new(write_half),
            pending,
            next_operation: AtomicU8::new(1),
            events,
            reader_task,
            connected_at,
        }
    }

    /// Subscribes to spontaneous messages from the earbuds.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    /// True once the connection has been closed.
    pub fn is_closed(&self) -> bool {
        self.reader_task.is_finished()
    }

    /// Sends a frame without waiting for a reply.
    pub async fn send(&self, command: u16, payload: Vec<u8>) -> Result<(), LinkError> {
        let frame = Frame::new(command, self.next_operation(), payload);
        self.write_frame(&frame).await
    }

    /// Sends a frame and waits for the reply carrying the same operation number.
    pub async fn request(
        &self,
        command: u16,
        payload: Vec<u8>,
        timeout: Duration,
    ) -> Result<Frame, LinkError> {
        let frame = Frame::new(command, self.next_operation(), payload);
        let (reply_tx, reply_rx) = oneshot::channel();
        self.lock_pending().insert(
            frame.operation,
            PendingRequest {
                command_id: frame.id(),
                reply: reply_tx,
            },
        );

        if let Err(error) = self.write_frame(&frame).await {
            self.lock_pending().remove(&frame.operation);
            return Err(error);
        }

        match tokio::time::timeout(timeout, reply_rx).await {
            Ok(Ok(reply)) => Ok(reply),
            Ok(Err(_)) => Err(LinkError::Disconnected), // reader task ended
            Err(_) => {
                self.lock_pending().remove(&frame.operation);
                Err(LinkError::Timeout(command::name(command)))
            }
        }
    }

    /// Operation numbers go from 1 to 255 and wrap around (0 is skipped).
    fn next_operation(&self) -> u8 {
        loop {
            let operation = self.next_operation.fetch_add(1, Ordering::Relaxed);
            if operation != 0 {
                return operation;
            }
        }
    }

    fn lock_pending(&self) -> std::sync::MutexGuard<'_, HashMap<u8, PendingRequest>> {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    async fn write_frame(&self, frame: &Frame) -> Result<(), LinkError> {
        if self.is_closed() {
            return Err(LinkError::Disconnected);
        }
        let bytes = frame.encode().map_err(|e| LinkError::Protocol(e.to_string()))?;
        log_frame("→", frame);

        let mut writer = self.writer.lock().await;
        loop {
            match writer.write_all(&bytes).await {
                Ok(()) => break,
                Err(e) if is_not_connected(&e) && self.connected_at.elapsed() < SETTLE_WINDOW => {
                    tokio::time::sleep(SETTLE_RETRY_DELAY).await;
                }
                Err(e) => return Err(e.into()),
            }
        }
        writer.flush().await?;
        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.reader_task.abort();
    }
}

/// Background task: reads the socket until it closes and dispatches frames.
async fn read_loop(
    mut socket: OwnedReadHalf,
    pending: PendingRequests,
    events: broadcast::Sender<Event>,
    connected_at: Instant,
) {
    let mut decoder = FrameDecoder::new();
    let mut buffer = [0u8; 1024];
    loop {
        match socket.read(&mut buffer).await {
            Ok(0) => break, // closed by the earbuds
            Ok(n) => decoder.push(&buffer[..n]),
            Err(e) if is_not_connected(&e) && connected_at.elapsed() < SETTLE_WINDOW => {
                tokio::time::sleep(SETTLE_RETRY_DELAY).await;
                continue;
            }
            Err(e) => {
                log::debug!("read stopped: {e}");
                break;
            }
        }
        while let Some(frame) = decoder.next_frame() {
            log_frame("←", &frame);
            dispatch(&pending, &events, frame);
        }
    }

    // Dropping the pending senders makes every waiting request fail with `Disconnected`.
    pending.lock().unwrap_or_else(|p| p.into_inner()).clear();
    let _ = events.send(Event::Disconnected);
}

/// Hands a frame to the request waiting for it, or broadcasts it as an event.
fn dispatch(pending: &PendingRequests, events: &broadcast::Sender<Event>, frame: Frame) {
    if let Some(request) = take_matching_request(pending, &frame) {
        let _ = request.reply.send(frame);
        return;
    }
    let _ = events.send(decode_event(frame));
}

fn take_matching_request(pending: &PendingRequests, frame: &Frame) -> Option<PendingRequest> {
    if frame.direction() == command::DIR_NOTIFY {
        return None;
    }
    let mut pending = pending.lock().unwrap_or_else(|p| p.into_inner());
    let matches = pending
        .get(&frame.operation)
        .is_some_and(|request| request.command_id == frame.id());
    if matches {
        pending.remove(&frame.operation)
    } else {
        None
    }
}

fn decode_event(frame: Frame) -> Event {
    let is = |cmd: u16| frame.id() == command::id(cmd);
    let decoded = if is(command::NOTIFY_BATTERY) || is(command::GET_BATTERY) {
        Battery::parse(&frame.payload).ok().map(Event::Battery)
    } else if is(command::NOTIFY_ANC) || is(command::GET_ANC) {
        AncMode::parse(&frame.payload).ok().map(Event::Anc)
    } else if is(command::NOTIFY_EAR_FIT_RESULT) {
        EarFit::parse(&frame.payload).ok().map(Event::EarFit)
    } else {
        None
    };
    decoded.unwrap_or(Event::Unsolicited(frame))
}

fn is_not_connected(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::NotConnected
}

fn log_frame(arrow: &str, frame: &Frame) {
    log::debug!(
        "{arrow} {:#06x} ({}) op={} {:02x?}",
        frame.command,
        command::name(frame.command),
        frame.operation,
        frame.payload
    );
}
