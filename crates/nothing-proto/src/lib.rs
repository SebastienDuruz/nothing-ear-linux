//! Control protocol of Nothing earbuds.
//!
//! This crate does no I/O: it only builds request frames and decodes
//! responses. The Bluetooth transport lives in the `nothing-link` crate.
//!
//! Layout:
//! - [`frame`]: the binary frame format (header, CRC, stream decoder);
//! - [`command`]: the list of known command codes;
//! - [`settings`]: one module per setting, each with its type, its decoder
//!   (`parse`) and its encoder (`encode`);
//! - [`model`]: known earbud models and the features they support.

pub mod command;
pub mod frame;
pub mod model;
pub mod settings;

pub use frame::{Frame, FrameDecoder};
pub use model::{Capabilities, Model};
pub use settings::*;

/// UUID of the proprietary service exposed by the earbuds. The control
/// channel is an RFCOMM socket advertised under this UUID.
pub const CONTROL_SERVICE_UUID: &str = "aeac4a03-dff5-498f-843a-34487cf133eb";
