use core::fmt::Write;
use heapless::String;

use crate::{error::Xtra, Error};

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

/// AT+CGNSCPY=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CopyXtraFile;

impl AtRequest for CopyXtraFile {
    type Response = (CopyResponse, GenericOk);
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CGNSCPY\r").unwrap();
        buf
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CopyResponse {
    Success = 0,
    FileDoesntExist = 1,
}

impl CopyResponse {
    pub fn success(&mut self) -> Result<(), Error> {
        match self {
            CopyResponse::Success => Ok(()),
            CopyResponse::FileDoesntExist => Err(Error::Xtra(Xtra::FileDoesntExist)),
        }
    }
}

impl AtParseLine for CopyResponse {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let line = line
            .strip_prefix("+CGNSCPY: ")
            .ok_or("Missing '+CGNSCPY: '")?;

        match line {
            "0" => Ok(CopyResponse::Success),
            "1" => Ok(CopyResponse::FileDoesntExist),
            _ => Err("Invalid response, expected 0 or 1".into()),
        }
    }
}

impl AtResponse for CopyResponse {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::CopyResponse(v) => Ok(v),
            _ => Err(code),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse() {
        let str = "+CGNSCPY: 0";
        let info = CopyResponse::from_line(str).expect("Parse CopyResponse");

        let expected = CopyResponse::Success;
        assert_eq!(expected, info);
    }
}
