use heapless::String;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

/// AT+CCLK
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GetTime;

impl AtRequest for GetTime {
    type Response = (CclkTime, GenericOk);
    fn encode(&self) -> String<256> {
        "AT+CCLK?\r".into()
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CclkTime {
    pub time: String<32>,
}

impl AtParseLine for CclkTime {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let line = line.strip_prefix("+CCLK: ").ok_or("Missing '+CCLK: '")?;

        Ok(CclkTime { time: line.into() })
    }
}

impl AtResponse for CclkTime {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::CclkTime(time) => Ok(time),
            _ => Err(code),
        }
    }
}
