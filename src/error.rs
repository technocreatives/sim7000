use embassy_time::TimeoutError;

use crate::at_command::{httptofs::StatusCode, SimError};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum Error {
    InvalidUtf8,
    BufferOverflow,
    Sim(SimError),
    Timeout,
    Serial,

    /// No default APN was set, and the network did not provide one.
    NoApn,
    Httptofs(StatusCode),
    Xtra(Xtra),
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Xtra {
    FileDoesntExist,
    NotEffective,
}

impl embedded_io_async::Error for Error {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        match self {
            Error::InvalidUtf8 => embedded_io_async::ErrorKind::InvalidData,
            Error::BufferOverflow => embedded_io_async::ErrorKind::OutOfMemory,
            Error::Sim(_) => embedded_io_async::ErrorKind::Other,
            Error::Timeout => embedded_io_async::ErrorKind::TimedOut,
            Error::Serial => embedded_io_async::ErrorKind::Other,
            Error::NoApn => embedded_io_async::ErrorKind::Other,
            Error::Httptofs(_) => embedded_io_async::ErrorKind::Other,
            Error::Xtra(_) => embedded_io_async::ErrorKind::Other,
        }
    }
}

impl From<TimeoutError> for Error {
    fn from(_: TimeoutError) -> Self {
        Error::Timeout
    }
}
