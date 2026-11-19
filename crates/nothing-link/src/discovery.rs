//! Finding paired Nothing earbuds through BlueZ.

use bluer::Uuid;
use serde::Serialize;

use crate::LinkError;

/// A paired device that exposes the Nothing control service.
#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub address: String,
    pub name: String,
    pub connected: bool,
}

fn control_service_uuid() -> Uuid {
    nothing_proto::CONTROL_SERVICE_UUID
        .parse()
        .expect("valid UUID constant")
}

/// Lists paired devices exposing the Nothing control service, connected
/// devices first.
pub async fn list_devices() -> Result<Vec<Candidate>, LinkError> {
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    let service = control_service_uuid();

    let mut candidates = Vec::new();
    for address in adapter.device_addresses().await? {
        let device = adapter.device(address)?;
        let uuids = device.uuids().await?.unwrap_or_default();
        if !uuids.contains(&service) {
            continue;
        }
        candidates.push(Candidate {
            address: address.to_string(),
            name: device.alias().await?,
            connected: device.is_connected().await?,
        });
    }
    candidates.sort_by_key(|candidate| !candidate.connected);
    Ok(candidates)
}

/// Returns the first connected candidate.
pub async fn first_connected() -> Result<Candidate, LinkError> {
    list_devices()
        .await?
        .into_iter()
        .find(|candidate| candidate.connected)
        .ok_or(LinkError::NotFound)
}

/// Returns the Bluetooth name of a paired device, or an empty string.
pub async fn name_of(address: &str) -> String {
    list_devices()
        .await
        .ok()
        .and_then(|list| list.into_iter().find(|c| c.address.eq_ignore_ascii_case(address)))
        .map(|candidate| candidate.name)
        .unwrap_or_default()
}
