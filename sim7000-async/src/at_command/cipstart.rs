use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
pub enum ConnectMode {
    Tcp,
    Udp,
}

/// AT+CIPSTART=...
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
pub struct Connect {
    /// Which connection slot to use (Multi-IP mode)
    pub number: usize,

    /// TCP or UDP
    pub mode: ConnectMode,

    /// IP or domain name
    #[cfg_attr(feature = "defmt", defmt(Debug2Format))]
    pub destination: String<100>,

    pub port: u16,
}

impl AtRequest for Connect {
    type Response = GenericOk; // TODO: should have its own type
    fn encode(&self) -> String<256> {
        let mode = match self.mode {
            ConnectMode::Tcp => "TCP",
            ConnectMode::Udp => "UDP",
        };

        let mut buf = String::new();
        write!(
            buf,
            "AT+CIPSTART={},{:?},{:?},\"{}\"\r",
            self.number, mode, self.destination, self.port
        )
        .unwrap();
        buf
    }
}
