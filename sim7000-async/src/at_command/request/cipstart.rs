use core::fmt::Write;
use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

pub enum ConnectMode {
    Tcp,
    Udp,
}

/// AT+CIPSTART=...
pub struct Connect {
    /// Which connection slot to use (Multi-IP mode)
    pub number: usize,

    /// TCP or UDP
    pub mode: ConnectMode,

    /// IP or domain name
    pub destination: String<100>,

    pub port: u16,
}

impl ATRequest for Connect {
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
