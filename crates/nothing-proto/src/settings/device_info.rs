//! Device information: firmware version and serial number.

use serde::{Deserialize, Serialize};

use super::ParseError;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub firmware: Option<String>,
    pub serial: Option<String>,
}

// Field types found in the device info lines.
const FIELD_FIRMWARE: &str = "2";
const FIELD_SERIAL: &str = "4";

impl DeviceInfo {
    /// Payload: one count byte, then text lines `part,field,value`
    /// (one set of lines per earbud and for the case). The first firmware
    /// and serial number found are kept.
    ///
    /// Example: `2,2,1.0.1.54` is the firmware of part 2 (left earbud).
    pub fn parse(payload: &[u8]) -> Result<Self, ParseError> {
        let text = payload.get(1..).ok_or(ParseError::TooShort("device info"))?;
        let text = String::from_utf8_lossy(text);

        let mut info = DeviceInfo::default();
        for line in text.lines() {
            let mut fields = line.trim().splitn(3, ',');
            let (_part, field, value) = (fields.next(), fields.next(), fields.next().unwrap_or("").trim());
            if value.is_empty() {
                continue;
            }
            match field {
                Some(FIELD_FIRMWARE) if info.firmware.is_none() => info.firmware = Some(value.to_owned()),
                Some(FIELD_SERIAL) if info.serial.is_none() => info.serial = Some(value.to_owned()),
                _ => {}
            }
        }
        Ok(info)
    }
}

/// Payload of the firmware reply: plain ASCII, for example `1.0.1.54`.
pub fn parse_firmware(payload: &[u8]) -> String {
    String::from_utf8_lossy(payload)
        .trim_matches('\0')
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_firmware_and_serial() {
        let payload = b"\x092,1,\n2,2,1.0.1.54\n2,4,SH10612525000179\n3,6,10CFF7EDB03C\n";
        let info = DeviceInfo::parse(payload).unwrap();
        assert_eq!(info.firmware.as_deref(), Some("1.0.1.54"));
        assert_eq!(info.serial.as_deref(), Some("SH10612525000179"));
    }
}
