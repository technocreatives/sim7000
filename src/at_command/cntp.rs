use core::fmt::Write;
use heapless::String;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

/// AT+CNTP=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SynchronizeNetworkTime {
    pub ntp_server: String<64>,
    pub timezone: u16,
    pub cid: u8,
}

/// AT+CNTP
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Execute;

impl AtRequest for SynchronizeNetworkTime {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(
            buf,
            "AT+CNTP={:?},{},{}\r",
            self.ntp_server, self.timezone, self.cid
        )
        .unwrap();
        buf
    }
}

impl AtRequest for Execute {
    type Response = (GenericOk, NetworkTime);
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CNTP\r").unwrap();
        buf
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NetworkTime {
    #[allow(dead_code)]
    time: String<32>,
}

impl AtParseLine for NetworkTime {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let line = line.strip_prefix("+CNTP: ").ok_or("Missing '+CNTP: '")?;
        let Some(line) = line.split_once(',') else {
            return match line {
                "61" => Err("Network error".into()),
                "62" => Err("DNS resolution error".into()),
                "63" => Err("Connection error".into()),
                "64" => Err("Service response error".into()),
                "65" => Err("Service response timeout".into()),
                _ => Err("Unexpected response".into()),
            }
        };

        Ok(NetworkTime {
            time: line.1.into(),
        })
    }
}

impl AtResponse for NetworkTime {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::NetworkTime(v) => Ok(v),
            _ => Err(code),
        }
    }
}
