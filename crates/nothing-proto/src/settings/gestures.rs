//! Touch gestures (pinches on the earbud stems).

use serde::{Deserialize, Serialize};

use super::ParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub const ALL: [Side; 2] = [Side::Left, Side::Right];

    pub fn wire_value(self) -> u8 {
        match self {
            Side::Left => 0x02,
            Side::Right => 0x03,
        }
    }

    pub fn from_wire_value(value: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|side| side.wire_value() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            Side::Left => "Left",
            Side::Right => "Right",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GestureKind {
    DoublePinch,
    TriplePinch,
    PinchAndHold,
    DoublePinchAndHold,
}

impl GestureKind {
    pub const ALL: [GestureKind; 4] = [
        GestureKind::DoublePinch,
        GestureKind::TriplePinch,
        GestureKind::PinchAndHold,
        GestureKind::DoublePinchAndHold,
    ];

    pub fn wire_value(self) -> u8 {
        match self {
            GestureKind::DoublePinch => 0x02,
            GestureKind::TriplePinch => 0x03,
            GestureKind::PinchAndHold => 0x07,
            GestureKind::DoublePinchAndHold => 0x09,
        }
    }

    pub fn from_wire_value(value: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.wire_value() == value)
    }

    pub fn label(self) -> &'static str {
        match self {
            GestureKind::DoublePinch => "Double pinch",
            GestureKind::TriplePinch => "Triple pinch",
            GestureKind::PinchAndHold => "Pinch and hold",
            GestureKind::DoublePinchAndHold => "Double pinch and hold",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GestureAction {
    NoAction,
    PlayPause,
    PreviousTrack,
    NextTrack,
    VoiceAssistant,
    VolumeUp,
    VolumeDown,
    /// Cycles through noise cancellation, transparency and off.
    NoiseControlCycleAll,
    /// Toggles between noise cancellation and off.
    NoiseControlAncOff,
    /// Toggles between transparency and off.
    NoiseControlTransparencyOff,
    /// Toggles between noise cancellation and transparency.
    NoiseControlAncTransparency,
    /// Action code we do not know.
    Other(u8),
}

impl GestureAction {
    /// Actions the user can pick.
    pub const SELECTABLE: [GestureAction; 11] = [
        GestureAction::NoAction,
        GestureAction::PlayPause,
        GestureAction::PreviousTrack,
        GestureAction::NextTrack,
        GestureAction::VoiceAssistant,
        GestureAction::VolumeUp,
        GestureAction::VolumeDown,
        GestureAction::NoiseControlCycleAll,
        GestureAction::NoiseControlAncOff,
        GestureAction::NoiseControlTransparencyOff,
        GestureAction::NoiseControlAncTransparency,
    ];

    pub fn wire_value(self) -> u8 {
        match self {
            GestureAction::NoAction => 0x01,
            GestureAction::PlayPause => 0x02,
            GestureAction::PreviousTrack => 0x08,
            GestureAction::NextTrack => 0x09,
            GestureAction::NoiseControlCycleAll => 0x0A,
            GestureAction::VoiceAssistant => 0x0B,
            GestureAction::VolumeUp => 0x12,
            GestureAction::VolumeDown => 0x13,
            GestureAction::NoiseControlAncOff => 0x14,
            GestureAction::NoiseControlTransparencyOff => 0x15,
            GestureAction::NoiseControlAncTransparency => 0x16,
            GestureAction::Other(value) => value,
        }
    }

    pub fn from_wire_value(value: u8) -> Self {
        Self::SELECTABLE
            .into_iter()
            .find(|action| action.wire_value() == value)
            .unwrap_or(GestureAction::Other(value))
    }

    pub fn label(self) -> String {
        match self {
            GestureAction::NoAction => "No action".into(),
            GestureAction::PlayPause => "Play / pause".into(),
            GestureAction::PreviousTrack => "Previous track".into(),
            GestureAction::NextTrack => "Next track".into(),
            GestureAction::VoiceAssistant => "Voice assistant".into(),
            GestureAction::VolumeUp => "Volume up".into(),
            GestureAction::VolumeDown => "Volume down".into(),
            GestureAction::NoiseControlCycleAll => "Noise: cancellation / transparency / off".into(),
            GestureAction::NoiseControlAncOff => "Noise: cancellation / off".into(),
            GestureAction::NoiseControlTransparencyOff => "Noise: transparency / off".into(),
            GestureAction::NoiseControlAncTransparency => "Noise: cancellation / transparency".into(),
            GestureAction::Other(value) => format!("Action {value:#04x}"),
        }
    }
}

/// One gesture binding: "on this side, this gesture does that".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gesture {
    pub side: Side,
    pub kind: GestureKind,
    pub action: GestureAction,
}

impl Gesture {
    /// Payload: entry count, then 4 bytes per gesture:
    /// `<side> 01 <gesture kind> <action>`.
    /// Entries with an unknown side or gesture kind are skipped.
    pub fn parse_list(payload: &[u8]) -> Result<Vec<Self>, ParseError> {
        let (&count, entries) = payload.split_first().ok_or(ParseError::TooShort("gestures"))?;
        let gestures = entries
            .chunks_exact(4)
            .take(usize::from(count))
            .filter_map(|entry| {
                Some(Gesture {
                    side: Side::from_wire_value(entry[0])?,
                    kind: GestureKind::from_wire_value(entry[2])?,
                    action: GestureAction::from_wire_value(entry[3]),
                })
            })
            .collect();
        Ok(gestures)
    }

    /// Payload to change one gesture: `01 <side> 01 <gesture kind> <action>`.
    pub fn encode(self) -> Vec<u8> {
        vec![
            0x01,
            self.side.wire_value(),
            0x01,
            self.kind.wire_value(),
            self.action.wire_value(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_gesture_list() {
        let payload = [
            0x08, 0x02, 0x01, 0x02, 0x09, 0x03, 0x01, 0x02, 0x09, 0x02, 0x01, 0x03, 0x08, 0x03, 0x01, 0x03,
            0x08, 0x02, 0x01, 0x07, 0x0B, 0x03, 0x01, 0x07, 0x16, 0x02, 0x01, 0x09, 0x01, 0x03, 0x01, 0x09,
            0x01,
        ];
        let gestures = Gesture::parse_list(&payload).unwrap();
        assert_eq!(gestures.len(), 8);
        assert_eq!(
            gestures[4],
            Gesture {
                side: Side::Left,
                kind: GestureKind::PinchAndHold,
                action: GestureAction::VoiceAssistant,
            }
        );
        assert_eq!(gestures[5].action, GestureAction::NoiseControlAncTransparency);
    }

    #[test]
    fn encodes_one_gesture() {
        let gesture = Gesture {
            side: Side::Left,
            kind: GestureKind::DoublePinch,
            action: GestureAction::NextTrack,
        };
        assert_eq!(gesture.encode(), vec![0x01, 0x02, 0x01, 0x02, 0x09]);
    }
}
