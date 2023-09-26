use heapless::String;

use crate::{error::Xtra, Error};

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

/// AT+CGNSCOLD=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GnssColdStart;

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum XtraStatus {
    Success = 0,
    DoesntExist = 1,
    NotEffective = 2,
}

impl XtraStatus {
    pub fn success(&mut self) -> Result<(), Error> {
        match self {
            XtraStatus::Success => Ok(()),
            XtraStatus::DoesntExist => Err(Error::Xtra(Xtra::FileDoesntExist)),
            XtraStatus::NotEffective => Err(Error::Xtra(Xtra::NotEffective)),
        }
    }
}

impl AtRequest for GnssColdStart {
    type Response = (GenericOk, XtraStatus);
    fn encode(&self) -> String<256> {
        "AT+CGNSCOLD\r".into()
    }
}

impl AtParseLine for XtraStatus {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let line = line
            .strip_prefix("+CGNSXTRA: ")
            .ok_or("Missing '+CGNSXTRA: '")?;

        match line {
            "0" => Ok(XtraStatus::Success),
            "1" => Ok(XtraStatus::DoesntExist),
            "2" => Ok(XtraStatus::NotEffective),
            _ => Err("Invalid response, expected 0, 1 or 2".into()),
        }
    }
}

impl AtResponse for XtraStatus {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::XtraStatus(v) => Ok(v),
            _ => Err(code),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse() {
        let str = "+CGNSXTRA: 0";
        let info = XtraStatus::from_line(str).expect("Parse XtraStatus");

        let expected = XtraStatus::Success;
        assert_eq!(expected, info);
    }
}
