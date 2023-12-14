use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CEREG=...
///
/// Configure network registration URC
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConfigureRegistrationUrc {
    /// Disable URC
    Disable = 0,

    /// Network registration URC
    EnableReg = 1,

    /// Network registration and location information URC
    EnableRegLocation = 2,
    //
    // EnableGprsTimeAndRau = 4,
}

/// AT+CEREG?
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GetRegistrationStatus;

impl AtRequest for ConfigureRegistrationUrc {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CEREG={}\r", *self as u8).unwrap();
        buf
    }
}

impl AtRequest for GetRegistrationStatus {
    // The actual response is generated as an URC
    type Response = GenericOk;

    fn encode(&self) -> String<256> {
        "AT+CEREG?\r".into()
    }
}
