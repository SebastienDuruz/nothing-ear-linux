//! One-off actions: "find my earbuds" ring and ear tip fit test.

use serde::{Deserialize, Serialize};

use super::ParseError;

// ---------------------------------------------------------------------------
// Ring ("find my earbuds")
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RingTarget {
    Both,
    Left,
    Right,
}

impl RingTarget {
    fn wire_value(self) -> u8 {
        match self {
            RingTarget::Both => 0x01,
            RingTarget::Left => 0x02,
            RingTarget::Right => 0x03,
        }
    }

    /// Payload: `<target> 01` to start ringing, `<target> 00` to stop.
    pub fn encode(self, ringing: bool) -> Vec<u8> {
        vec![self.wire_value(), u8::from(ringing)]
    }
}

// ---------------------------------------------------------------------------
// Ear tip fit test
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FitResult {
    Good,
    Poor,
}

/// Result of the ear tip fit test, per side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EarFit {
    pub left: FitResult,
    pub right: FitResult,
}

impl EarFit {
    /// Payload: one byte per side (left, right); 0 means a good seal.
    pub fn parse(payload: &[u8]) -> Result<Self, ParseError> {
        let result = |value: u8| {
            if value == 0 {
                FitResult::Good
            } else {
                FitResult::Poor
            }
        };
        match payload {
            [left, right, ..] => Ok(Self {
                left: result(*left),
                right: result(*right),
            }),
            _ => Err(ParseError::TooShort("ear tip fit test")),
        }
    }

    /// Payload that starts the test.
    pub fn encode_start() -> Vec<u8> {
        vec![0x01]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fit_result() {
        let fit = EarFit::parse(&[0x00, 0x01]).unwrap();
        assert_eq!(fit.left, FitResult::Good);
        assert_eq!(fit.right, FitResult::Poor);
    }
}
