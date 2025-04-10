use core::fmt::Write;
use heapless::String;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

/// AT+CMGR=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ReadSms {
    pub index: u8,
}

impl AtRequest for ReadSms {
    type Response = (SmsMessage, GenericOk);
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CMGR={}\r", self.index).unwrap();
        buf
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SmsMessage {
    pub sender: String<20>,
    pub message: String<160>,
}

impl AtParseLine for SmsMessage {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;

        if message != "+CMGR" {
            return Err("Missing +CMGR prefix".into());
        }

        let (_status, rest) = rest.split_once(',').ok_or("Missing ','")?;
        let (sender, _) = rest.split_once(',').ok_or("Missing ','")?;

        Ok(Self {
            sender: sender.trim_matches('\"').into(),
            message: "".into(),
        })
    }
}

// impl AtParseLine for SmsMessage {
//     fn from_line(line: &str) -> Result<Self, AtParseErr> {
//         // This is pretty scuffed, but the way this currently works we need to filter out at commands
//         // not all start with '+' and contain ':'
//         if line.starts_with('+') && line.contains(':') {
//             return Err("Invalid line".into());
//         }

//         Ok(Self {
//             message: line.into(),
//         })
//     }
// }

impl AtResponse for SmsMessage {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::SmsMessage(sms) => Ok(sms),
            _ => Err(code),
        }
    }
}
// impl AtResponse for SmsInfo {
//     fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
//         match code {
//             ResponseCode::SmsInfo(sms) => Ok(sms),
//             _ => Err(code),
//         }
//     }
// }
