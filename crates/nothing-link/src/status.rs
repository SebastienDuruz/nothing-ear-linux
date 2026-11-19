use nothing_proto::{AncMode, BassEnhance, Battery, CustomEq, EqPreset, Gesture};
use serde::Serialize;

/// Snapshot of every setting. A field is `None` when the model does not
/// support it or when it could not be read.
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub name: String,
    pub address: String,
    pub model: String,
    pub model_code: String,
    pub firmware: Option<String>,
    pub serial: Option<String>,
    pub battery: Option<Battery>,
    pub anc: Option<AncMode>,
    pub eq_preset: Option<EqPreset>,
    pub custom_eq: Option<CustomEq>,
    pub bass: Option<BassEnhance>,
    pub low_latency: Option<bool>,
    pub in_ear_detection: Option<bool>,
    pub gestures: Option<Vec<Gesture>>,
}
