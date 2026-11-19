//! Equalizer: presets and the 3-band custom equalizer.

use serde::{Deserialize, Serialize};

use super::ParseError;

// ---------------------------------------------------------------------------
// Presets
// ---------------------------------------------------------------------------

/// Equalizer preset.
///
/// To be confirmed: community sources disagree on the order of
/// "more bass" (1) and "more treble" (2). [`EqPreset::Custom`] (5) makes the
/// earbuds use the custom equalizer: writing the custom bands alone leaves
/// the current preset active. Any other code is kept as [`EqPreset::Other`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EqPreset {
    Balanced,
    MoreBass,
    MoreTreble,
    Voice,
    /// The 3-band custom equalizer ([`CustomEq`]).
    Custom,
    Other(u8),
}

impl EqPreset {
    /// Presets the user can pick.
    pub const SELECTABLE: [EqPreset; 4] = [
        EqPreset::Balanced,
        EqPreset::MoreBass,
        EqPreset::MoreTreble,
        EqPreset::Voice,
    ];

    pub fn wire_value(self) -> u8 {
        match self {
            EqPreset::Balanced => 0,
            EqPreset::MoreBass => 1,
            EqPreset::MoreTreble => 2,
            EqPreset::Voice => 3,
            EqPreset::Custom => 5,
            EqPreset::Other(value) => value,
        }
    }

    pub fn from_wire_value(value: u8) -> Self {
        match value {
            0 => EqPreset::Balanced,
            1 => EqPreset::MoreBass,
            2 => EqPreset::MoreTreble,
            3 => EqPreset::Voice,
            5 => EqPreset::Custom,
            other => EqPreset::Other(other),
        }
    }

    pub fn label(self) -> String {
        match self {
            EqPreset::Balanced => "Balanced".into(),
            EqPreset::MoreBass => "More bass".into(),
            EqPreset::MoreTreble => "More treble".into(),
            EqPreset::Voice => "Voice".into(),
            EqPreset::Custom => "Custom".into(),
            EqPreset::Other(value) => format!("Preset {value}"),
        }
    }

    /// Payload: one byte, the preset.
    pub fn parse(payload: &[u8]) -> Result<Self, ParseError> {
        payload
            .first()
            .map(|&value| Self::from_wire_value(value))
            .ok_or(ParseError::TooShort("EQ preset"))
    }

    /// Payload: `<preset> 00`.
    pub fn encode(self) -> Vec<u8> {
        vec![self.wire_value(), 0x00]
    }
}

// ---------------------------------------------------------------------------
// Custom equalizer
// ---------------------------------------------------------------------------

/// Lowest gain accepted by the custom equalizer, in dB.
pub const CUSTOM_EQ_MIN_DB: f32 = -6.0;
/// Highest gain accepted by the custom equalizer, in dB.
pub const CUSTOM_EQ_MAX_DB: f32 = 6.0;

/// 3-band custom equalizer, gains in dB.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CustomEq {
    pub bass_db: f32,
    pub mid_db: f32,
    pub treble_db: f32,
}

/// One band of the custom equalizer (a peaking filter).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EqBand {
    /// Filter type byte, copied as-is from the official app.
    pub kind: u8,
    pub gain_db: f32,
    pub frequency_hz: f32,
    pub q: f32,
}

// Payload layout:
//   [0]         band count
//   [1..5]      overall gain (pre-attenuation to avoid clipping)
//   then 13 bytes per band: kind (1), gain (4), frequency (4), Q (4)
//   then 9 zero bytes when writing
const HEADER_LEN: usize = 5;
const BAND_LEN: usize = 13;
const TRAILER_LEN: usize = 9;

impl CustomEq {
    /// The three bands, in the order the official app sends them:
    /// mid (980 Hz), treble (3.5 kHz), bass (140 Hz).
    fn bands(&self) -> [EqBand; 3] {
        let clamp = |gain: f32| gain.clamp(CUSTOM_EQ_MIN_DB, CUSTOM_EQ_MAX_DB);
        [
            EqBand {
                kind: 0x01,
                gain_db: clamp(self.mid_db),
                frequency_hz: 980.0,
                q: 0.66,
            },
            EqBand {
                kind: 0x02,
                gain_db: clamp(self.treble_db),
                frequency_hz: 3500.0,
                q: 1.0,
            },
            EqBand {
                kind: 0x00,
                gain_db: clamp(self.bass_db),
                frequency_hz: 140.0,
                q: 0.8,
            },
        ]
    }

    pub fn encode(&self) -> Vec<u8> {
        let bands = self.bands();
        // The overall gain lowers the output by the highest boost, so that
        // boosting a band does not clip.
        let highest_gain = bands.iter().map(|b| b.gain_db).fold(0.0_f32, f32::max);

        let mut payload = Vec::with_capacity(HEADER_LEN + bands.len() * BAND_LEN + TRAILER_LEN);
        payload.push(bands.len() as u8);
        payload.extend_from_slice(&encode_overall_gain(-highest_gain));
        for band in bands {
            payload.push(band.kind);
            payload.extend_from_slice(&encode_gain(band.gain_db));
            payload.extend_from_slice(&band.frequency_hz.to_le_bytes());
            payload.extend_from_slice(&band.q.to_le_bytes());
        }
        payload.extend_from_slice(&[0; TRAILER_LEN]);
        payload
    }

    /// Finds each band by its frequency. An empty reply (no bands) means
    /// every gain is 0 dB.
    pub fn parse(payload: &[u8]) -> Result<Self, ParseError> {
        let mut eq = CustomEq::default();
        for band in parse_eq_bands(payload)? {
            match band.frequency_hz {
                f if f < 500.0 => eq.bass_db = band.gain_db,
                f if f < 2000.0 => eq.mid_db = band.gain_db,
                _ => eq.treble_db = band.gain_db,
            }
        }
        Ok(eq)
    }
}

/// Decodes the list of bands of a custom equalizer reply.
pub fn parse_eq_bands(payload: &[u8]) -> Result<Vec<EqBand>, ParseError> {
    let too_short = ParseError::TooShort("custom EQ");
    let count = usize::from(*payload.first().ok_or(too_short.clone())?);

    let mut bands = Vec::with_capacity(count);
    for index in 0..count {
        let start = HEADER_LEN + index * BAND_LEN;
        let raw = payload.get(start..start + BAND_LEN).ok_or(too_short.clone())?;
        bands.push(EqBand {
            kind: raw[0],
            gain_db: decode_gain([raw[1], raw[2], raw[3], raw[4]]),
            frequency_hz: f32::from_le_bytes([raw[5], raw[6], raw[7], raw[8]]),
            q: f32::from_le_bytes([raw[9], raw[10], raw[11], raw[12]]),
        });
    }
    Ok(bands)
}

// Gains are little-endian `f32` with one quirk copied from the official app:
// when the three most significant bytes are zero, the sign is stored in
// bit 7 of the last byte instead of the usual sign bit.

fn encode_gain(value: f32) -> [u8; 4] {
    let mut be = value.to_be_bytes();
    if value != 0.0 && be[0] == 0 && be[1] == 0 && be[2] == 0 {
        be[3] |= 0x80;
    }
    [be[3], be[2], be[1], be[0]]
}

fn decode_gain(le: [u8; 4]) -> f32 {
    let mut be = [le[3], le[2], le[1], le[0]];
    if be[0] == 0 && be[1] == 0 && be[2] == 0 && be[3] & 0x80 != 0 {
        be[3] &= 0x7F;
        return -f32::from_be_bytes(be);
    }
    f32::from_be_bytes(be)
}

/// The overall gain is never positive: zero is sent as "-0".
fn encode_overall_gain(value: f32) -> [u8; 4] {
    if value >= 0.0 {
        [0x00, 0x00, 0x00, 0x80]
    } else {
        encode_gain(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_preset_wire_value() {
        // Verified on B171: `F010 05` selects the custom equalizer.
        assert_eq!(EqPreset::Custom.encode(), vec![0x05, 0x00]);
        assert_eq!(EqPreset::parse(&[0x05]).unwrap(), EqPreset::Custom);
        assert_eq!(EqPreset::parse(&[0x07]).unwrap(), EqPreset::Other(7));
    }

    #[test]
    fn flat_eq_matches_official_app_template() {
        let expected: [u8; 53] = [
            0x03, 0x00, 0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x75, 0x44, 0xC3, 0xF5,
            0x28, 0x3F, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x5A, 0x45, 0x00, 0x00, 0x80, 0x3F, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x43, 0xCD, 0xCC, 0x4C, 0x3F, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(CustomEq::default().encode(), expected);
    }

    #[test]
    fn custom_eq_round_trip() {
        let eq = CustomEq {
            bass_db: 4.5,
            mid_db: -2.0,
            treble_db: 1.0,
        };
        let payload = eq.encode();
        assert_eq!(CustomEq::parse(&payload).unwrap(), eq);
        // The overall gain is minus the highest boost.
        assert_eq!(
            decode_gain([payload[1], payload[2], payload[3], payload[4]]),
            -4.5
        );
    }

    #[test]
    fn empty_reply_means_flat() {
        // Real reply from a B171 that never had a custom EQ set.
        assert_eq!(CustomEq::parse(&[0u8; 109]).unwrap(), CustomEq::default());
    }
}
