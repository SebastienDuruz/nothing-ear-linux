//! Earbud settings: one module per setting.
//!
//! Every setting follows the same pattern:
//! - a Rust type describing the value (`AncMode`, `Battery`, ...);
//! - `parse(payload)` decoding the payload of the earbuds' reply;
//! - `encode()` building the payload of a write request.
//!
//! Payload examples in the tests were captured from a real pair of
//! Nothing Ear (B171).

mod anc;
mod battery;
mod device_info;
mod equalizer;
mod gestures;
mod switches;
mod tools;

pub use anc::AncMode;
pub use battery::{Battery, BatteryLevel};
pub use device_info::{DeviceInfo, parse_firmware};
pub use equalizer::{CUSTOM_EQ_MAX_DB, CUSTOM_EQ_MIN_DB, CustomEq, EqBand, EqPreset, parse_eq_bands};
pub use gestures::{Gesture, GestureAction, GestureKind, Side};
pub use switches::{BassEnhance, FLAG_IN_EAR_DETECTION, FeatureFlags, encode_low_latency, parse_low_latency};
pub use tools::{EarFit, FitResult, RingTarget};

use thiserror::Error;

/// Error returned when a reply payload cannot be decoded.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("reply too short for \"{0}\"")]
    TooShort(&'static str),
    #[error("unknown value {value:#04x} for \"{setting}\"")]
    UnknownValue { setting: &'static str, value: u8 },
}
