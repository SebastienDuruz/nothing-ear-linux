//! Simple settings: on/off switches and bass enhance.

use serde::{Deserialize, Serialize};

use super::ParseError;

// ---------------------------------------------------------------------------
// Feature flags (in-ear detection, ...)
// ---------------------------------------------------------------------------

/// Id of the "in-ear detection" option in [`FeatureFlags`].
pub const FLAG_IN_EAR_DETECTION: u8 = 0x01;

/// List of `(option id, value)` pairs returned by command `0xC00E`.
/// Only option 1 (in-ear detection) is identified so far; the others are
/// kept so they can be inspected.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureFlags(pub Vec<(u8, u8)>);

impl FeatureFlags {
    /// Payload: pair count, then `(id, value)` pairs.
    pub fn parse(payload: &[u8]) -> Result<Self, ParseError> {
        let (&count, pairs) = payload.split_first().ok_or(ParseError::TooShort("options"))?;
        let flags = pairs
            .chunks_exact(2)
            .take(usize::from(count))
            .map(|pair| (pair[0], pair[1]))
            .collect();
        Ok(Self(flags))
    }

    pub fn get(&self, id: u8) -> Option<bool> {
        self.0
            .iter()
            .find(|(flag, _)| *flag == id)
            .map(|(_, value)| *value != 0)
    }

    pub fn in_ear_detection(&self) -> Option<bool> {
        self.get(FLAG_IN_EAR_DETECTION)
    }

    /// Payload to change one option: `01 <id> <value>`.
    pub fn encode_one(id: u8, enabled: bool) -> Vec<u8> {
        vec![0x01, id, u8::from(enabled)]
    }
}

// ---------------------------------------------------------------------------
// Low latency mode
// ---------------------------------------------------------------------------

/// Payload: one byte, 1 = on, 2 = off.
pub fn parse_low_latency(payload: &[u8]) -> Result<bool, ParseError> {
    match payload.first() {
        Some(1) => Ok(true),
        Some(0 | 2) => Ok(false),
        Some(&value) => Err(ParseError::UnknownValue {
            setting: "low latency",
            value,
        }),
        None => Err(ParseError::TooShort("low latency")),
    }
}

/// Payload: `01 00` to turn on, `02 00` to turn off.
pub fn encode_low_latency(enabled: bool) -> Vec<u8> {
    vec![if enabled { 1 } else { 2 }, 0x00]
}

// ---------------------------------------------------------------------------
// Bass enhance
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BassEnhance {
    pub enabled: bool,
    /// Level from 1 to [`BassEnhance::MAX_LEVEL`].
    pub level: u8,
}

impl BassEnhance {
    pub const MAX_LEVEL: u8 = 5;

    /// Payload: `<enabled> <level × 2>`.
    pub fn parse(payload: &[u8]) -> Result<Self, ParseError> {
        match payload {
            [enabled, doubled_level, ..] => Ok(Self {
                enabled: *enabled != 0,
                level: (doubled_level / 2).clamp(1, Self::MAX_LEVEL),
            }),
            _ => Err(ParseError::TooShort("bass enhance")),
        }
    }

    pub fn encode(self) -> Vec<u8> {
        let level = self.level.clamp(1, Self::MAX_LEVEL);
        vec![u8::from(self.enabled), level * 2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_flags_contain_in_ear_detection() {
        let payload = [
            0x09, 0x01, 0x01, 0x02, 0x01, 0x07, 0x01, 0x09, 0x01, 0x0A, 0x01, 0x0B, 0x00, 0x0E, 0x01, 0x12,
            0x01, 0x15, 0x01,
        ];
        let flags = FeatureFlags::parse(&payload).unwrap();
        assert_eq!(flags.0.len(), 9);
        assert_eq!(flags.in_ear_detection(), Some(true));
        assert_eq!(flags.get(0x0B), Some(false));
    }

    #[test]
    fn bass_enhance_level_is_doubled_on_the_wire() {
        assert_eq!(
            BassEnhance::parse(&[0x01, 0x06]).unwrap(),
            BassEnhance {
                enabled: true,
                level: 3
            }
        );
        assert_eq!(
            BassEnhance {
                enabled: true,
                level: 5
            }
            .encode(),
            vec![1, 10]
        );
    }

    #[test]
    fn low_latency_uses_two_for_off() {
        assert!(!parse_low_latency(&[0x02]).unwrap());
        assert_eq!(encode_low_latency(true), vec![1, 0]);
    }
}
