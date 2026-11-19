//! Noise control (active noise cancellation and transparency).

use serde::{Deserialize, Serialize};

use super::ParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AncMode {
    Off,
    Transparency,
    Adaptive,
    High,
    Mid,
    Low,
}

impl AncMode {
    /// All modes, in the order shown in the apps.
    pub const ALL: [AncMode; 6] = [
        AncMode::Off,
        AncMode::Transparency,
        AncMode::Adaptive,
        AncMode::High,
        AncMode::Mid,
        AncMode::Low,
    ];

    pub fn wire_value(self) -> u8 {
        match self {
            AncMode::High => 1,
            AncMode::Mid => 2,
            AncMode::Low => 3,
            AncMode::Adaptive => 4,
            AncMode::Off => 5,
            AncMode::Transparency => 7,
        }
    }

    pub fn from_wire_value(value: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.wire_value() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            AncMode::Off => "Off",
            AncMode::Transparency => "Transparency",
            AncMode::Adaptive => "Noise cancellation: adaptive",
            AncMode::High => "Noise cancellation: high",
            AncMode::Mid => "Noise cancellation: mid",
            AncMode::Low => "Noise cancellation: low",
        }
    }

    /// Payload: triplets `(kind, value, 0)`. The triplet of kind 1 holds the mode.
    /// Example: `01 07 00 02 01 00` means "transparency".
    ///
    /// If no triplet of kind 1 is found, the second byte is used, as other
    /// clients do for the noise control notification (`0xE003`).
    pub fn parse(payload: &[u8]) -> Result<Self, ParseError> {
        const MODE_KIND: u8 = 1;
        let value = payload
            .chunks(3)
            .find(|triplet| triplet.len() >= 2 && triplet[0] == MODE_KIND)
            .map(|triplet| triplet[1])
            .or_else(|| payload.get(1).copied())
            .ok_or(ParseError::TooShort("noise control"))?;
        Self::from_wire_value(value).ok_or(ParseError::UnknownValue {
            setting: "noise control",
            value,
        })
    }

    /// Payload: `01 <mode> 00`.
    pub fn encode(self) -> Vec<u8> {
        vec![0x01, self.wire_value(), 0x00]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_transparency() {
        let mode = AncMode::parse(&[0x01, 0x07, 0x00, 0x02, 0x01, 0x00]).unwrap();
        assert_eq!(mode, AncMode::Transparency);
    }

    #[test]
    fn encodes_high() {
        assert_eq!(AncMode::High.encode(), vec![1, 1, 0]);
    }
}
