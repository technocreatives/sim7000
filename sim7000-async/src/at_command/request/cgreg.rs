use core::fmt::Write;
use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

/// AT+CGREG=...
///
/// Configure network registration URC
#[repr(u8)]
#[derive(Clone, Copy)]
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

/// AT+CGREG?
pub struct GetRegistrationStatus;

impl ATRequest for ConfigureRegistrationUrc {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CGREG={}\r", *self as u8).unwrap();
        buf
    }
}

impl ATRequest for GetRegistrationStatus {
    // The actual response is generated as an URC
    type Response = GenericOk;

    fn encode(&self) -> String<256> {
        "AT+CGREG?\r".into()
    }
}
