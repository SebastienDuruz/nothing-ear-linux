//! High-level API: one method per setting.

use std::time::Duration;

use bluer::Address;
use bluer::rfcomm::{SocketAddr, Stream};
use nothing_proto::{
    AncMode, BassEnhance, Battery, CustomEq, DeviceInfo, EarFit, EqPreset, FeatureFlags, Frame, Gesture,
    Model, RingTarget, command, model, settings,
};
use tokio::sync::broadcast;

use crate::{Client, DEFAULT_TIMEOUT, Event, LinkError, Status, discovery};

/// RFCOMM channels on which the control service has been seen.
/// Nothing Ear (B171): 15. CMF Buds 2: 16.
pub const KNOWN_CHANNELS: [u8; 2] = [15, 16];

/// Connection attempts per channel. Right after a disconnection the earbuds
/// may briefly refuse a new connection.
const CONNECT_ATTEMPTS: usize = 3;
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(600);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(6);
/// The ear tip fit test plays a sound for several seconds.
const EAR_FIT_TIMEOUT: Duration = Duration::from_secs(25);

#[derive(Debug, Clone, Default)]
pub struct ConnectOptions {
    /// Bluetooth address to use. Default: the first connected Nothing device.
    pub address: Option<String>,
    /// RFCOMM channel to use. Default: try [`KNOWN_CHANNELS`].
    pub channel: Option<u8>,
}

/// Connected earbuds, ready to take commands.
pub struct Earbuds {
    client: Client,
    pub address: String,
    pub name: String,
    pub model: Model,
    pub channel: u8,
}

impl Earbuds {
    /// Finds the earbuds and opens the control channel.
    pub async fn connect(options: &ConnectOptions) -> Result<Self, LinkError> {
        let (address, name) = match &options.address {
            Some(address) => (address.clone(), discovery::name_of(address).await),
            None => {
                let found = discovery::first_connected().await?;
                (found.address, found.name)
            }
        };
        let bt_address: Address = address
            .parse()
            .map_err(|_| LinkError::BadAddress(address.clone()))?;

        let channels = match options.channel {
            Some(channel) => vec![channel],
            None => KNOWN_CHANNELS.to_vec(),
        };
        let (stream, channel) = open_control_channel(bt_address, &channels).await?;
        log::info!("control channel open on {address} (RFCOMM {channel})");

        Ok(Self {
            client: Client::new(stream),
            model: model::from_bluetooth_name(&name),
            address,
            name,
            channel,
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Subscribes to battery and noise control changes made on the earbuds.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.client.subscribe()
    }

    /// Sends a raw command and returns the reply (for debugging).
    pub async fn raw(&self, command: u16, payload: Vec<u8>) -> Result<Frame, LinkError> {
        self.client.request(command, payload, DEFAULT_TIMEOUT).await
    }

    /// Sends a read request and returns the reply payload.
    async fn read(&self, command: u16) -> Result<Vec<u8>, LinkError> {
        Ok(self
            .client
            .request(command, Vec::new(), DEFAULT_TIMEOUT)
            .await?
            .payload)
    }

    /// Sends a write request and waits for the acknowledgement.
    async fn write(&self, command: u16, payload: Vec<u8>) -> Result<(), LinkError> {
        self.client
            .request(command, payload, DEFAULT_TIMEOUT)
            .await
            .map(|_ack| ())
    }

    // --- Reads ---------------------------------------------------------------

    pub async fn battery(&self) -> Result<Battery, LinkError> {
        Ok(Battery::parse(&self.read(command::GET_BATTERY).await?)?)
    }

    pub async fn anc(&self) -> Result<AncMode, LinkError> {
        Ok(AncMode::parse(&self.read(command::GET_ANC).await?)?)
    }

    pub async fn eq_preset(&self) -> Result<EqPreset, LinkError> {
        Ok(EqPreset::parse(&self.read(command::GET_EQ_PRESET).await?)?)
    }

    pub async fn custom_eq(&self) -> Result<CustomEq, LinkError> {
        Ok(CustomEq::parse(&self.read(command::GET_CUSTOM_EQ).await?)?)
    }

    pub async fn bass(&self) -> Result<BassEnhance, LinkError> {
        Ok(BassEnhance::parse(&self.read(command::GET_BASS).await?)?)
    }

    pub async fn low_latency(&self) -> Result<bool, LinkError> {
        Ok(settings::parse_low_latency(
            &self.read(command::GET_LOW_LATENCY).await?,
        )?)
    }

    pub async fn feature_flags(&self) -> Result<FeatureFlags, LinkError> {
        Ok(FeatureFlags::parse(
            &self.read(command::GET_FEATURE_FLAGS).await?,
        )?)
    }

    pub async fn in_ear_detection(&self) -> Result<bool, LinkError> {
        self.feature_flags()
            .await?
            .in_ear_detection()
            .ok_or(LinkError::Protocol(
                "in-ear detection missing from the reply".into(),
            ))
    }

    pub async fn gestures(&self) -> Result<Vec<Gesture>, LinkError> {
        Ok(Gesture::parse_list(&self.read(command::GET_GESTURES).await?)?)
    }

    pub async fn firmware(&self) -> Result<String, LinkError> {
        Ok(settings::parse_firmware(&self.read(command::GET_FIRMWARE).await?))
    }

    pub async fn device_info(&self) -> Result<DeviceInfo, LinkError> {
        Ok(DeviceInfo::parse(&self.read(command::GET_DEVICE_INFO).await?)?)
    }

    /// Reads every setting supported by the model. A setting that cannot be
    /// read is left to `None` (and logged as a warning).
    pub async fn status(&self) -> Status {
        let caps = self.model.capabilities;
        Status {
            name: self.name.clone(),
            address: self.address.clone(),
            model: self.model.name.to_owned(),
            model_code: self.model.code.to_owned(),
            firmware: read_if(true, self.firmware()).await,
            serial: read_if(true, self.device_info())
                .await
                .and_then(|info| info.serial),
            battery: read_if(true, self.battery()).await,
            anc: read_if(caps.anc, self.anc()).await,
            eq_preset: read_if(caps.eq_presets, self.eq_preset()).await,
            custom_eq: read_if(caps.custom_eq, self.custom_eq()).await,
            bass: read_if(caps.bass_enhance, self.bass()).await,
            low_latency: read_if(caps.low_latency, self.low_latency()).await,
            in_ear_detection: read_if(caps.in_ear_detection, self.in_ear_detection()).await,
            gestures: read_if(caps.gestures, self.gestures()).await,
        }
    }

    // --- Writes --------------------------------------------------------------

    pub async fn set_anc(&self, mode: AncMode) -> Result<(), LinkError> {
        self.write(command::SET_ANC, mode.encode()).await
    }

    pub async fn set_eq_preset(&self, preset: EqPreset) -> Result<(), LinkError> {
        self.write(command::SET_EQ_PRESET, preset.encode()).await
    }

    /// Writes the custom bands, then selects the custom preset: without it,
    /// the earbuds store the bands but keep playing the previous preset.
    pub async fn set_custom_eq(&self, eq: CustomEq) -> Result<(), LinkError> {
        self.write(command::SET_CUSTOM_EQ, eq.encode()).await?;
        self.set_eq_preset(EqPreset::Custom).await
    }

    pub async fn set_bass(&self, bass: BassEnhance) -> Result<(), LinkError> {
        self.write(command::SET_BASS, bass.encode()).await
    }

    pub async fn set_low_latency(&self, enabled: bool) -> Result<(), LinkError> {
        self.write(command::SET_LOW_LATENCY, settings::encode_low_latency(enabled))
            .await
    }

    pub async fn set_in_ear_detection(&self, enabled: bool) -> Result<(), LinkError> {
        let payload = FeatureFlags::encode_one(settings::FLAG_IN_EAR_DETECTION, enabled);
        self.write(command::SET_FEATURE_FLAG, payload).await
    }

    pub async fn set_gesture(&self, gesture: Gesture) -> Result<(), LinkError> {
        self.write(command::SET_GESTURE, gesture.encode()).await
    }

    // --- Tools ---------------------------------------------------------------

    /// Starts or stops ringing. The sound is very loud: the earbuds must
    /// not be in the user's ears.
    pub async fn ring(&self, target: RingTarget, ringing: bool) -> Result<(), LinkError> {
        // Stopping is done one side at a time.
        let targets = match (target, ringing) {
            (RingTarget::Both, false) => vec![RingTarget::Left, RingTarget::Right],
            _ => vec![target],
        };
        for target in targets {
            match self.write(command::SET_RING, target.encode(ringing)).await {
                // Some firmwares do not acknowledge this command.
                Ok(()) | Err(LinkError::Timeout(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// Runs the ear tip fit test and waits for its result. The earbuds must
    /// be worn; a sound plays during the test.
    pub async fn ear_fit_test(&self) -> Result<EarFit, LinkError> {
        // Subscribe before starting, so the result cannot be missed.
        let mut events = self.subscribe();

        let _ = self.read(command::GET_EAR_FIT_PREPARE).await; // reply not needed
        self.client
            .send(command::SET_EAR_FIT_START, EarFit::encode_start())
            .await?;

        let wait_for_result = async {
            loop {
                match events.recv().await {
                    Ok(Event::EarFit(result)) => return Ok(result),
                    Ok(Event::Disconnected) | Err(broadcast::error::RecvError::Closed) => {
                        return Err(LinkError::Disconnected);
                    }
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                }
            }
        };
        tokio::time::timeout(EAR_FIT_TIMEOUT, wait_for_result)
            .await
            .map_err(|_| LinkError::Timeout(command::name(command::NOTIFY_EAR_FIT_RESULT)))?
    }
}

/// Tries each channel a few times and returns the first stream that opens.
async fn open_control_channel(address: Address, channels: &[u8]) -> Result<(Stream, u8), LinkError> {
    let mut errors = Vec::new();
    for &channel in channels {
        for attempt in 1..=CONNECT_ATTEMPTS {
            if attempt > 1 {
                tokio::time::sleep(CONNECT_RETRY_DELAY).await;
            }
            let target = SocketAddr::new(address, channel);
            match tokio::time::timeout(CONNECT_TIMEOUT, Stream::connect(target)).await {
                Ok(Ok(stream)) => return Ok((stream, channel)),
                Ok(Err(error)) => {
                    log::debug!("channel {channel}, attempt {attempt}: {error}");
                    errors.push(format!("channel {channel}: {error}"));
                    if error.kind() == std::io::ErrorKind::ConnectionRefused {
                        break; // nothing listens on this channel, try the next one
                    }
                }
                Err(_) => errors.push(format!("channel {channel}: timed out")),
            }
        }
    }
    errors.dedup();
    Err(LinkError::ChannelUnavailable(errors.join("; ")))
}

/// Runs `read` only if the feature is supported, and turns errors into `None`.
async fn read_if<T>(supported: bool, read: impl Future<Output = Result<T, LinkError>>) -> Option<T> {
    if !supported {
        return None;
    }
    read.await.inspect_err(|error| log::warn!("{error}")).ok()
}
