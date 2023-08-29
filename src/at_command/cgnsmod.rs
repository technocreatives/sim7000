use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WorkMode {
    Stop = 0,
    Start = 1,
    StartOutsideUs = 2,
}

/// AT+CGNSMOD=GLONASS, BEIDOU, GALILIEAN
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetGnssWorkModeSet {
    pub glonass: WorkMode,
    pub beidou: WorkMode,
    pub galilean: WorkMode,
}

/// AT+CGNSMOD?
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GetGnssWorkModeSet;

impl AtRequest for SetGnssWorkModeSet {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(
            buf,
            "AT+CGNSMOD=1,{},{},{}\r",
            self.glonass as u8, self.beidou as u8, self.galilean as u8
        )
        .unwrap();
        buf
    }
}

impl AtRequest for GetGnssWorkModeSet {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        "AT+CGNSMOD?\r".into()
    }
}
