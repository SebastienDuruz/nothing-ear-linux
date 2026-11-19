//! Connection to Nothing earbuds through BlueZ.
//!
//! Audio goes through the standard Bluetooth profiles (A2DP/HFP), handled by
//! PipeWire. Settings go through a proprietary RFCOMM channel, which this
//! crate opens.
//!
//! Layout:
//! - [`discovery`]: find paired Nothing earbuds;
//! - [`Earbuds`]: open the control channel and read/write settings;
//! - [`Client`]: low-level request/reply matching on the RFCOMM socket;
//! - [`Status`]: a snapshot of every setting.
//!
//! ```no_run
//! # async fn demo() -> Result<(), nothing_link::LinkError> {
//! let earbuds = nothing_link::Earbuds::connect(&Default::default()).await?;
//! println!("{:?}", earbuds.battery().await?);
//! # Ok(()) }
//! ```

mod client;
pub mod discovery;
mod earbuds;
mod error;
mod status;

pub use client::{Client, DEFAULT_TIMEOUT, Event};
pub use discovery::{Candidate, list_devices};
pub use earbuds::{ConnectOptions, Earbuds, KNOWN_CHANNELS};
pub use error::LinkError;
pub use status::Status;
