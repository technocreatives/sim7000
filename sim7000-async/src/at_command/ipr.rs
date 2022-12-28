use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BaudRate {
    Hz0 = 0,
    Hz300 = 300,
    Hz600 = 600,
    Hz1200 = 1200,
    Hz2400 = 2400,
    Hz4800 = 4800,
    Hz9600 = 9600,
    Hz19200 = 19200,
    Hz38400 = 38400,
    Hz57600 = 57600,
    Hz115200 = 115200,
    Hz230400 = 230400,
    Hz921600 = 921600,
    Hz2000000 = 2000000,
    Hz2900000 = 2900000,
    Hz3000000 = 3000000,
    Hz3200000 = 3200000,
    Hz3686400 = 3686400,
    Hz4000000 = 4000000,
}

/// AT+IPR=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetBaudRate(pub BaudRate);

impl AtRequest for SetBaudRate {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+IPR={}\r", self.0 as u32).unwrap();
        buf
    }
}
