use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConnectMode {
    Tcp,
    Udp,
}

/// AT+CIPSTART=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Connect {
    /// Which connection slot to use (Multi-IP mode)
    pub number: usize,

    /// TCP or UDP
    pub mode: ConnectMode,

    /// IP or domain name
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
            "AT+CIPSTART={},{mode:?},{:?},\"{}\"\r",
            self.number, self.destination, self.port
        )
        .unwrap();
        buf
    }
}
