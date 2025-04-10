use core::fmt::Write;
use heapless::String;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

/// AT+CMGF=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetSmsMessageFormat(pub SmsMessageFormat);

impl AtRequest for SetSmsMessageFormat {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CMGF={}\r", self.0 as u8).unwrap();
        buf
    }
}

/// AT+CMGF?
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GetSmsMessageFormat;

impl AtRequest for GetSmsMessageFormat {
    type Response = (SmsMessageFormat, GenericOk);
    fn encode(&self) -> String<256> {
        "AT+CMGF?\r".into()
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SmsMessageFormat {
    Pdu = 0,
    Text = 1,
}

impl AtParseLine for SmsMessageFormat {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;

        if message != "+CMGF" {
            return Err("Missing +CMGF prefix".into());
        }

        match rest {
            "0" => Ok(SmsMessageFormat::Pdu),
            "1" => Ok(SmsMessageFormat::Text),
            _ => Err("Invalid SMS message format".into()),
        }
    }
}

impl AtResponse for SmsMessageFormat {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::SmsMessageFormat(format) => Ok(format),
            _ => Err(code),
        }
    }
}
