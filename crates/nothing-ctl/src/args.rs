//! Command-line arguments.
//!
//! The `*Arg` enums mirror protocol types with names that are convenient to
//! type in a shell. Each one converts into its protocol type with `From`.

use clap::{Parser, Subcommand, ValueEnum};
use nothing_proto::{AncMode, EqPreset, GestureAction, GestureKind, RingTarget, Side};

#[derive(Parser)]
#[command(name = "nothing-ctl", version, about = "Control Nothing earbuds from Linux")]
pub struct Cli {
    /// Bluetooth address of the earbuds (default: first connected Nothing device)
    #[arg(short, long, global = true)]
    pub device: Option<String>,
    /// RFCOMM control channel (default: auto-detect)
    #[arg(short, long, global = true)]
    pub channel: Option<u8>,
    /// Print results as JSON
    #[arg(long, global = true)]
    pub json: bool,
    /// Log every frame sent and received
    #[arg(short, long, global = true)]
    pub verbose: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List paired Nothing earbuds
    Devices,
    /// Show every setting
    Status,
    /// Show battery levels
    Battery,
    /// Show or change noise control
    Anc { mode: Option<AncArg> },
    /// Show or change the EQ preset
    Eq { preset: Option<EqArg> },
    /// Show or change the custom EQ (gains in dB, from -6 to +6); changing it also selects it
    CustomEq {
        #[arg(long, allow_hyphen_values = true)]
        bass: Option<f32>,
        #[arg(long, allow_hyphen_values = true)]
        mid: Option<f32>,
        #[arg(long, allow_hyphen_values = true)]
        treble: Option<f32>,
    },
    /// Show or change bass enhance: "off", or a level from 1 to 5
    Bass { level: Option<String> },
    /// Show or change low latency mode
    Latency { state: Option<Switch> },
    /// Show or change in-ear detection (pause when an earbud is removed)
    InEar { state: Option<Switch> },
    /// Show gesture bindings
    Gestures,
    /// Change the action of one gesture
    Gesture {
        side: SideArg,
        kind: GestureKindArg,
        action: GestureActionArg,
    },
    /// Ring the earbuds to find them (take them out of your ears first!)
    Ring {
        #[arg(default_value = "both")]
        target: RingArg,
        /// Stop ringing
        #[arg(long)]
        stop: bool,
        /// Confirm that the earbuds are not in your ears
        #[arg(long)]
        yes: bool,
    },
    /// Ear tip fit test (wear the earbuds; a sound plays)
    FitTest,
    /// Send a raw command, in hex (debugging)
    Raw {
        /// Command code, for example c007
        command: String,
        /// Payload, for example 010700
        payload: Option<String>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Switch {
    On,
    Off,
}

impl From<Switch> for bool {
    fn from(switch: Switch) -> bool {
        matches!(switch, Switch::On)
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum AncArg {
    Off,
    Transparency,
    Adaptive,
    High,
    Mid,
    Low,
}

impl From<AncArg> for AncMode {
    fn from(arg: AncArg) -> Self {
        match arg {
            AncArg::Off => AncMode::Off,
            AncArg::Transparency => AncMode::Transparency,
            AncArg::Adaptive => AncMode::Adaptive,
            AncArg::High => AncMode::High,
            AncArg::Mid => AncMode::Mid,
            AncArg::Low => AncMode::Low,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum EqArg {
    Balanced,
    MoreBass,
    MoreTreble,
    Voice,
    /// The custom equalizer (see `custom-eq`)
    Custom,
}

impl From<EqArg> for EqPreset {
    fn from(arg: EqArg) -> Self {
        match arg {
            EqArg::Balanced => EqPreset::Balanced,
            EqArg::MoreBass => EqPreset::MoreBass,
            EqArg::MoreTreble => EqPreset::MoreTreble,
            EqArg::Voice => EqPreset::Voice,
            EqArg::Custom => EqPreset::Custom,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum SideArg {
    Left,
    Right,
}

impl From<SideArg> for Side {
    fn from(arg: SideArg) -> Self {
        match arg {
            SideArg::Left => Side::Left,
            SideArg::Right => Side::Right,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum GestureKindArg {
    DoublePinch,
    TriplePinch,
    Hold,
    DoubleHold,
}

impl From<GestureKindArg> for GestureKind {
    fn from(arg: GestureKindArg) -> Self {
        match arg {
            GestureKindArg::DoublePinch => GestureKind::DoublePinch,
            GestureKindArg::TriplePinch => GestureKind::TriplePinch,
            GestureKindArg::Hold => GestureKind::PinchAndHold,
            GestureKindArg::DoubleHold => GestureKind::DoublePinchAndHold,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum GestureActionArg {
    None,
    PlayPause,
    Previous,
    Next,
    Assistant,
    VolumeUp,
    VolumeDown,
    /// Cycle noise cancellation / transparency / off
    NoiseCycle,
    /// Toggle noise cancellation / off
    NoiseAncOff,
    /// Toggle transparency / off
    NoiseTransparencyOff,
    /// Toggle noise cancellation / transparency
    NoiseAncTransparency,
}

impl From<GestureActionArg> for GestureAction {
    fn from(arg: GestureActionArg) -> Self {
        match arg {
            GestureActionArg::None => GestureAction::NoAction,
            GestureActionArg::PlayPause => GestureAction::PlayPause,
            GestureActionArg::Previous => GestureAction::PreviousTrack,
            GestureActionArg::Next => GestureAction::NextTrack,
            GestureActionArg::Assistant => GestureAction::VoiceAssistant,
            GestureActionArg::VolumeUp => GestureAction::VolumeUp,
            GestureActionArg::VolumeDown => GestureAction::VolumeDown,
            GestureActionArg::NoiseCycle => GestureAction::NoiseControlCycleAll,
            GestureActionArg::NoiseAncOff => GestureAction::NoiseControlAncOff,
            GestureActionArg::NoiseTransparencyOff => GestureAction::NoiseControlTransparencyOff,
            GestureActionArg::NoiseAncTransparency => GestureAction::NoiseControlAncTransparency,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum RingArg {
    Both,
    Left,
    Right,
}

impl From<RingArg> for RingTarget {
    fn from(arg: RingArg) -> Self {
        match arg {
            RingArg::Both => RingTarget::Both,
            RingArg::Left => RingTarget::Left,
            RingArg::Right => RingTarget::Right,
        }
    }
}
