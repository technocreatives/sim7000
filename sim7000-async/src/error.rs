use embassy_time::TimeoutError;

use crate::at_command::SimError;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    InvalidUtf8,
    BufferOverflow,
    Sim(SimError),
    Timeout,
    Serial,

    /// No default APN was set, and the network did not provide one.
    NoApn,
}

impl embedded_io::Error for Error {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

impl From<TimeoutError> for Error {
    fn from(_: TimeoutError) -> Self {
        Error::Timeout
    }
}
