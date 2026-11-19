//! Known command codes.
//!
//! A command is a `u16`. Its high byte is the direction:
//! - `0xC0` read request from the host, answered with `0x40`;
//! - `0xF0` write request from the host, acknowledged with `0x70`;
//! - `0xE0` notification sent by the earbuds on their own.
//!
//! Its low byte is the id of the setting. A reply or acknowledgement keeps
//! the id of the request: `0xC007` (read battery) is answered by `0x4007`.
//!
//! See `docs/PROTOCOL.md` for which commands were verified on real earbuds.

/// Direction byte of a read request.
pub const DIR_GET: u8 = 0xC0;
/// Direction byte of a write request.
pub const DIR_SET: u8 = 0xF0;
/// Direction byte of a reply to a read request.
pub const DIR_REPLY: u8 = 0x40;
/// Direction byte of an acknowledgement to a write request.
pub const DIR_ACK: u8 = 0x70;
/// Direction byte of a notification sent spontaneously by the earbuds.
pub const DIR_NOTIFY: u8 = 0xE0;

// --- Read requests -----------------------------------------------------------

pub const GET_DEVICE_INFO: u16 = 0xC006;
pub const GET_BATTERY: u16 = 0xC007;
/// Prepares the ear tip fit test (sent just before `SET_EAR_FIT_START`).
pub const GET_EAR_FIT_PREPARE: u16 = 0xC00A;
/// Generic on/off options. Option 1 is in-ear detection.
pub const GET_FEATURE_FLAGS: u16 = 0xC00E;
pub const GET_GESTURES: u16 = 0xC018;
pub const GET_ANC: u16 = 0xC01E;
pub const GET_EQ_PRESET: u16 = 0xC01F;
/// No reply on Nothing Ear (B171).
pub const GET_PERSONALIZED_ANC: u16 = 0xC020;
pub const GET_LOW_LATENCY: u16 = 0xC041;
pub const GET_FIRMWARE: u16 = 0xC042;
pub const GET_CUSTOM_EQ: u16 = 0xC044;
pub const GET_BASS: u16 = 0xC04E;

// --- Write requests ----------------------------------------------------------

pub const SET_RING: u16 = 0xF002;
pub const SET_GESTURE: u16 = 0xF003;
pub const SET_FEATURE_FLAG: u16 = 0xF004;
pub const SET_ANC: u16 = 0xF00F;
pub const SET_EQ_PRESET: u16 = 0xF010;
pub const SET_PERSONALIZED_ANC: u16 = 0xF011;
pub const SET_EAR_FIT_START: u16 = 0xF014;
pub const SET_LOW_LATENCY: u16 = 0xF040;
/// Note: same id (0x41) as `GET_LOW_LATENCY`, but a different direction.
pub const SET_CUSTOM_EQ: u16 = 0xF041;
pub const SET_BASS: u16 = 0xF051;

// --- Notifications -----------------------------------------------------------

pub const NOTIFY_BATTERY: u16 = 0xE001;
pub const NOTIFY_ANC: u16 = 0xE003;
pub const NOTIFY_EAR_FIT_RESULT: u16 = 0xE00D;

/// Returns the id (low byte) of a command.
pub const fn id(command: u16) -> u8 {
    (command & 0x00FF) as u8
}

/// Human-readable name of a command, used in logs and error messages.
/// Replies and acknowledgements get the name of their request.
pub fn name(command: u16) -> &'static str {
    // Notification ids overlap with request ids, so match them first.
    match command {
        NOTIFY_BATTERY => return "battery notification",
        NOTIFY_ANC => return "noise control notification",
        NOTIFY_EAR_FIT_RESULT => return "ear tip fit result",
        _ => {}
    }
    match id(command) {
        0x02 => "ring",
        0x03 => "set gesture",
        0x04 => "set option",
        0x06 => "device info",
        0x07 => "battery",
        0x0A => "prepare ear tip fit test",
        0x0D => "ear tip fit result",
        0x0E => "options",
        0x0F => "set noise control",
        0x10 => "set EQ preset",
        0x11 => "set personalized noise control",
        0x14 => "start ear tip fit test",
        0x18 => "gestures",
        0x1E => "noise control",
        0x1F => "EQ preset",
        0x20 => "personalized noise control",
        0x40 => "set low latency",
        0x41 => "low latency / set custom EQ",
        0x42 => "firmware",
        0x44 => "custom EQ",
        0x4E => "bass enhance",
        0x51 => "set bass enhance",
        _ => "unknown command",
    }
}
