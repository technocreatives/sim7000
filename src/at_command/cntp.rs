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
        "AT+CNTP\r".into()
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SyncNtpStatusCode {
    Success = 1,
    NetworkError = 61,
    DnsResolutionError = 62,
    ConnectionError = 63,
    ServiceResponseError = 64,
    ServiceResponseTimeout = 65,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NetworkTime {
    pub code: SyncNtpStatusCode,
    pub time: Option<String<32>>,
}

impl AtParseLine for NetworkTime {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let line = line.strip_prefix("+CNTP: ").ok_or("Missing '+CNTP: '")?;

        let (code, time) = match line.split_once(',') {
            Some((code, time)) => (code, Some(time.into())),
            None => (line, None),
        };

        let code = match code {
            "1" => SyncNtpStatusCode::Success,
            "61" => SyncNtpStatusCode::NetworkError,
            "62" => SyncNtpStatusCode::DnsResolutionError,
            "63" => SyncNtpStatusCode::ConnectionError,
            "64" => SyncNtpStatusCode::ServiceResponseError,
            "65" => SyncNtpStatusCode::ServiceResponseTimeout,
            _ => return Err("Unexpected response".into()),
        };

        Ok(NetworkTime { code, time })
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
