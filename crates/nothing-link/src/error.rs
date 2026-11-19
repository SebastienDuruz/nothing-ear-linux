use nothing_proto::ParseError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LinkError {
    #[error("Bluetooth: {0}")]
    Bluetooth(#[from] bluer::Error),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("no Nothing earbuds connected (check that Bluetooth is on and the earbuds are connected)")]
    NotFound,
    #[error("invalid Bluetooth address: {0}")]
    BadAddress(String),
    #[error("cannot open the control channel: {0}")]
    ChannelUnavailable(String),
    #[error("no reply from the earbuds ({0})")]
    Timeout(&'static str),
    #[error("connection closed by the earbuds")]
    Disconnected,
    #[error("unreadable reply: {0}")]
    Parse(#[from] ParseError),
    #[error("protocol error: {0}")]
    Protocol(String),
}

impl LinkError {
    /// True when the RFCOMM connection is gone and must be reopened.
    pub fn is_connection_lost(&self) -> bool {
        matches!(self, LinkError::Disconnected | LinkError::Io(_))
    }
}
