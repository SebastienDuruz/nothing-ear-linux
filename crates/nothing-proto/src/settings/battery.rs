//! Battery levels.

use serde::{Deserialize, Serialize};

use super::ParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatteryLevel {
    pub percent: u8,
    pub charging: bool,
}

/// Battery levels of each part. The case is only reported while its lid is
/// open with at least one earbud inside.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Battery {
    pub left: Option<BatteryLevel>,
    pub right: Option<BatteryLevel>,
    pub case: Option<BatteryLevel>,
}

// Part ids used in the payload.
const BOTH_BUDS: u8 = 0x01; // older models report a single value for both buds
const LEFT: u8 = 0x02;
const RIGHT: u8 = 0x03;
const CASE: u8 = 0x04;

impl Battery {
    /// Payload: entry count, then one `(part, value)` pair per entry.
    /// In `value`, bit 7 means "charging" and bits 0..6 are the percentage.
    pub fn parse(payload: &[u8]) -> Result<Self, ParseError> {
        let (&count, pairs) = payload.split_first().ok_or(ParseError::TooShort("battery"))?;

        let mut battery = Battery::default();
        for pair in pairs.chunks_exact(2).take(usize::from(count)) {
            let (part, value) = (pair[0], pair[1]);
            let level = BatteryLevel {
                percent: value & 0x7F,
                charging: value & 0x80 != 0,
            };
            if level.percent > 100 {
                continue; // invalid reading
            }
            match part {
                BOTH_BUDS => {
                    battery.left = Some(level);
                    battery.right = Some(level);
                }
                LEFT => battery.left = Some(level),
                RIGHT => battery.right = Some(level),
                CASE => battery.case = Some(level),
                _ => {}
            }
        }
        Ok(battery)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_buds_with_case_closed() {
        let battery = Battery::parse(&[0x02, 0x02, 0x5A, 0x03, 0x5A]).unwrap();
        assert_eq!(
            battery.left,
            Some(BatteryLevel {
                percent: 90,
                charging: false
            })
        );
        assert_eq!(
            battery.right,
            Some(BatteryLevel {
                percent: 90,
                charging: false
            })
        );
        assert_eq!(battery.case, None);
    }

    #[test]
    fn charging_bud_and_case() {
        let battery = Battery::parse(&[0x03, 0x02, 0xE4, 0x03, 0x64, 0x04, 0x32]).unwrap();
        assert_eq!(
            battery.left,
            Some(BatteryLevel {
                percent: 100,
                charging: true
            })
        );
        assert_eq!(
            battery.case,
            Some(BatteryLevel {
                percent: 50,
                charging: false
            })
        );
    }
}
