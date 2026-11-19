//! Frames exchanged over the RFCOMM control channel.
//!
//! ```text
//! byte  0      start of frame, always 0x55
//! bytes 1..=2  control field, u16 little-endian (0x0160; bit 0x20 = "has CRC")
//! bytes 3..=4  command, u16 little-endian (high byte = direction, low byte = id)
//! bytes 5..=6  payload length, u16 little-endian
//! byte  7      operation number, echoed back by the earbuds in their reply
//! bytes 8..    payload
//! last 2 bytes CRC-16/MODBUS, little-endian, over everything before it
//! ```

use thiserror::Error;

/// Start-of-frame marker.
pub const START_OF_FRAME: u8 = 0x55;
/// Control field value used by the official app.
pub const CONTROL: u16 = 0x0160;
/// Bit of the control field telling that the frame ends with a CRC.
const CONTROL_HAS_CRC: u16 = 0x0020;
/// Header size, i.e. everything before the payload.
pub const HEADER_LEN: usize = 8;
/// Size of the trailing CRC.
pub const CRC_LEN: usize = 2;
/// Largest frame we accept. Protects against garbage on the stream.
pub const MAX_FRAME_LEN: usize = 4096;

/// A frame, either decoded from the earbuds or about to be sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Full command code (direction + id), for example `0xC007`.
    pub command: u16,
    /// Operation number. The earbuds copy it into their reply, which lets us
    /// match replies to requests.
    pub operation: u8,
    pub payload: Vec<u8>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EncodeError {
    #[error("payload too large ({0} bytes)")]
    PayloadTooLarge(usize),
}

impl Frame {
    pub fn new(command: u16, operation: u8, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            command,
            operation,
            payload: payload.into(),
        }
    }

    /// Command id (low byte), without the direction.
    pub fn id(&self) -> u8 {
        crate::command::id(self.command)
    }

    /// Direction (high byte): `0xC0` read, `0xF0` write, `0x40` reply,
    /// `0x70` write acknowledgement, `0xE0` notification.
    pub fn direction(&self) -> u8 {
        (self.command >> 8) as u8
    }

    /// Serialises the frame, CRC included.
    pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
        let payload_len = self.payload.len();
        if HEADER_LEN + payload_len + CRC_LEN > MAX_FRAME_LEN {
            return Err(EncodeError::PayloadTooLarge(payload_len));
        }

        let mut bytes = Vec::with_capacity(HEADER_LEN + payload_len + CRC_LEN);
        bytes.push(START_OF_FRAME);
        bytes.extend_from_slice(&CONTROL.to_le_bytes());
        bytes.extend_from_slice(&self.command.to_le_bytes());
        bytes.extend_from_slice(&(payload_len as u16).to_le_bytes());
        bytes.push(self.operation);
        bytes.extend_from_slice(&self.payload);

        let crc = crc16_modbus(&bytes);
        bytes.extend_from_slice(&crc.to_le_bytes());
        Ok(bytes)
    }
}

/// CRC-16/MODBUS (reflected polynomial 0xA001, initial value 0xFFFF).
pub fn crc16_modbus(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= u16::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xA001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

/// Rebuilds frames from a byte stream that may arrive in arbitrary chunks.
///
/// Usage: call [`push`](Self::push) with each chunk read from the socket,
/// then call [`next_frame`](Self::next_frame) until it returns `None`.
///
/// Stray bytes and frames with a wrong CRC are skipped. The number of
/// rejected frames is available through [`dropped`](Self::dropped).
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
    dropped: usize,
}

impl FrameDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends bytes received from the socket.
    pub fn push(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// Number of frames rejected so far (bad CRC or impossible length).
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    /// Returns the next complete frame, or `None` if more bytes are needed.
    pub fn next_frame(&mut self) -> Option<Frame> {
        loop {
            self.skip_to_start_of_frame();
            if self.buffer.len() < HEADER_LEN {
                return None;
            }

            let control = u16::from_le_bytes([self.buffer[1], self.buffer[2]]);
            let command = u16::from_le_bytes([self.buffer[3], self.buffer[4]]);
            let payload_len = usize::from(u16::from_le_bytes([self.buffer[5], self.buffer[6]]));
            let has_crc = control & CONTROL_HAS_CRC != 0;
            let frame_len = HEADER_LEN + payload_len + if has_crc { CRC_LEN } else { 0 };

            if frame_len > MAX_FRAME_LEN {
                // Not a real frame start: drop this 0x55 and look for the next one.
                self.buffer.remove(0);
                self.dropped += 1;
                continue;
            }
            if self.buffer.len() < frame_len {
                return None;
            }

            let raw: Vec<u8> = self.buffer.drain(..frame_len).collect();
            if has_crc && !crc_matches(&raw) {
                self.dropped += 1;
                continue;
            }
            return Some(Frame {
                command,
                operation: raw[7],
                payload: raw[HEADER_LEN..HEADER_LEN + payload_len].to_vec(),
            });
        }
    }

    /// Drops every byte before the next start-of-frame marker.
    fn skip_to_start_of_frame(&mut self) {
        match self.buffer.iter().position(|&b| b == START_OF_FRAME) {
            Some(start) => {
                self.buffer.drain(..start);
            }
            None => self.buffer.clear(),
        }
    }
}

/// Checks the CRC stored in the last two bytes of a raw frame.
fn crc_matches(raw: &[u8]) -> bool {
    let (body, crc) = raw.split_at(raw.len() - CRC_LEN);
    crc16_modbus(body) == u16::from_le_bytes([crc[0], crc[1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_check_value() {
        // Standard check value of CRC-16/MODBUS.
        assert_eq!(crc16_modbus(b"123456789"), 0x4B37);
    }

    #[test]
    fn encodes_battery_request() {
        let bytes = Frame::new(0xC007, 1, Vec::new()).encode().unwrap();
        assert_eq!(&bytes[..8], &[0x55, 0x60, 0x01, 0x07, 0xC0, 0x00, 0x00, 0x01]);
        assert_eq!(bytes.len(), 10);
    }

    #[test]
    fn decodes_frames_split_across_chunks() {
        let battery = Frame::new(0x4007, 3, vec![0x02, 0x02, 0x5A, 0x03, 0x5A]);
        let anc = Frame::new(0x401E, 4, vec![0x01, 0x07, 0x00]);
        let mut stream = vec![0x00, 0x13]; // garbage before the first frame
        stream.extend(battery.encode().unwrap());
        stream.extend(anc.encode().unwrap());

        let mut decoder = FrameDecoder::new();
        let mut decoded = Vec::new();
        for chunk in stream.chunks(3) {
            decoder.push(chunk);
            while let Some(frame) = decoder.next_frame() {
                decoded.push(frame);
            }
        }
        assert_eq!(decoded, vec![battery, anc]);
        assert_eq!(decoder.dropped(), 0);
    }

    #[test]
    fn skips_frame_with_bad_crc() {
        let mut stream = Frame::new(0x4041, 1, vec![0x02]).encode().unwrap();
        let last = stream.len() - 1;
        stream[last] ^= 0xFF; // corrupt the CRC
        let good = Frame::new(0x4042, 2, b"1.0.1.54".to_vec());
        stream.extend(good.encode().unwrap());

        let mut decoder = FrameDecoder::new();
        decoder.push(&stream);
        assert_eq!(decoder.next_frame(), Some(good));
        assert_eq!(decoder.next_frame(), None);
        assert_eq!(decoder.dropped(), 1);
    }

    #[test]
    fn decodes_real_capture() {
        // Battery reply captured from a pair of Nothing Ear (B171).
        let mut raw = vec![
            0x55, 0x60, 0x01, 0x07, 0x40, 0x05, 0x00, 0x01, 0x02, 0x02, 0x5A, 0x03, 0x5A,
        ];
        let crc = crc16_modbus(&raw);
        raw.extend(crc.to_le_bytes());

        let mut decoder = FrameDecoder::new();
        decoder.push(&raw);
        let frame = decoder.next_frame().unwrap();
        assert_eq!(frame.command, 0x4007);
        assert_eq!(frame.direction(), 0x40);
        assert_eq!(frame.id(), 0x07);
    }
}
