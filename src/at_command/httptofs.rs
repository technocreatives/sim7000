use core::fmt::Write;
use heapless::String;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum StatusCode {
    Continue,
    Ok,
    PartialContent,
    BadRequest,
    NotFound,
    RequestTimeout,
    InternalServerError,
    NotHttpPdu,
    NetworkError,
    NoMemory,
    DnsError,
    StackBusy,
    SslContinue,
    OtherErrors,
}

/// AT+HTTPTOFS=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DownloadToFileSystem {
    pub url: String<64>,
    pub file_path: String<32>,
}

impl AtRequest for DownloadToFileSystem {
    type Response = (GenericOk, DownloadInfo);
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+HTTPTOFS={:?},{:?}\r", self.url, self.file_path).unwrap();
        buf
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DownloadInfo {
    pub status_code: StatusCode,
    data_length: u64,
}

impl StatusCode {
    pub fn success(&mut self) -> Result<(), Self> {
        match self {
            StatusCode::Ok => Ok(()),
            _ => Err(*self),
        }
    }
}

impl AtParseLine for DownloadInfo {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let line = line
            .strip_prefix("+HTTPTOFS: ")
            .ok_or("Missing '+HTTPTOFS: '")?;
        let (status_code, data_length) = match line.split_once(',') {
            Some(t) => t,
            None => (line, "0"),
        };

        let status_code = match status_code {
            "100" => StatusCode::Continue,
            "200" => StatusCode::Ok,
            "206" => StatusCode::PartialContent,
            "400" => StatusCode::BadRequest,
            "404" => StatusCode::NotFound,
            "408" => StatusCode::RequestTimeout,
            "500" => StatusCode::InternalServerError,
            "600" => StatusCode::NotHttpPdu,
            "601" => StatusCode::NetworkError,
            "602" => StatusCode::NoMemory,
            "603" => StatusCode::DnsError,
            "604" => StatusCode::StackBusy,
            "620" => StatusCode::SslContinue,
            _ => StatusCode::OtherErrors,
        };

        Ok(DownloadInfo {
            status_code,
            data_length: data_length.parse::<u64>().unwrap_or(0),
        })
    }
}

impl AtResponse for DownloadInfo {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::DownloadInfo(v) => Ok(v),
            _ => Err(code),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse() {
        let str = "+HTTPTOFS: 601,0";
        let info = DownloadInfo::from_line(str).expect("Parse DownloadInfo");

        let expected = DownloadInfo {
            status_code: StatusCode::NetworkError,
            data_length: 0,
        };
        assert_eq!(expected, info);
    }
}
