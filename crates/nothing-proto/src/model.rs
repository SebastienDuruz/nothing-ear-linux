//! Known earbud models and the features they support.
//!
//! To add a model: create a `Model` constant below and teach
//! [`from_bluetooth_name`] to recognise its Bluetooth name.

use serde::{Deserialize, Serialize};

/// Features available on a model. The apps only show what is supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub anc: bool,
    pub adaptive_anc: bool,
    pub eq_presets: bool,
    pub custom_eq: bool,
    pub bass_enhance: bool,
    pub in_ear_detection: bool,
    pub low_latency: bool,
    pub gestures: bool,
    pub ear_fit_test: bool,
    pub ring: bool,
    pub personalized_anc: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Nothing's internal model code, for example `B171`.
    pub code: &'static str,
    pub name: &'static str,
    pub capabilities: Capabilities,
}

/// Nothing Ear (2024). Every read was checked on a real pair, except
/// personalized ANC, which does not answer.
pub const NOTHING_EAR: Model = Model {
    code: "B171",
    name: "Nothing Ear",
    capabilities: Capabilities {
        anc: true,
        adaptive_anc: true,
        eq_presets: true,
        custom_eq: true,
        bass_enhance: true,
        in_ear_detection: true,
        low_latency: true,
        gestures: true,
        ear_fit_test: true,
        ring: true,
        personalized_anc: false,
    },
};

/// Conservative feature set for a model we do not know.
pub const GENERIC: Model = Model {
    code: "unknown",
    name: "Nothing / CMF earbuds",
    capabilities: Capabilities {
        anc: true,
        adaptive_anc: false,
        eq_presets: true,
        custom_eq: false,
        bass_enhance: false,
        in_ear_detection: true,
        low_latency: true,
        gestures: true,
        ear_fit_test: false,
        ring: true,
        personalized_anc: false,
    },
};

/// Guesses the model from the name the earbuds advertise over Bluetooth.
pub fn from_bluetooth_name(name: &str) -> Model {
    // Other models add a suffix: "Nothing Ear (2)", "Nothing Ear (a)", ...
    if name.trim().eq_ignore_ascii_case("Nothing Ear") {
        NOTHING_EAR
    } else {
        GENERIC
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_b171_only_by_exact_name() {
        assert_eq!(from_bluetooth_name("Nothing Ear").code, "B171");
        assert_eq!(from_bluetooth_name("Nothing Ear (2)").code, "unknown");
    }
}
