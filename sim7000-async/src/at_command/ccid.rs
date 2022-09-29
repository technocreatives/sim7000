use heapless::String;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

pub struct ShowIccid;

impl AtRequest for ShowIccid {
    type Response = (Iccid, GenericOk);
    fn encode(&self) -> String<256> {
        "AT+CCID\r".into()
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Iccid {
    pub country: u8,
    pub issuer: u8,
    pub account: u64,
    // pub checksum: u8,
}

impl AtParseLine for Iccid {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        //  "89 88 28 0666001104843 8"
        //  "89 01 26 0862291477114 f"
        if line.len() != 20 {
            return Err("Invalid length".into());
        }

        if &line[..2] != "89" {
            return Err("Invalid MII".into());
        }

        let country = line[2..4].parse()?;
        let issuer = line[4..6].parse()?;
        let account = line[6..19].parse()?;

        // TODO: this seems incorrect, but the ICCID standard scares me
        let _checksum: u8 = u8::from_str_radix(&line[18..19], 16)?;

        Ok(Iccid {
            country,
            issuer,
            account,
        })
    }
}

impl AtResponse for Iccid {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::Iccid(iccid) => Ok(iccid),
            _ => Err(code),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_iccid() {
        let valid_iccds = ["89882806660011048438", "8901260862291477114f"];

        for iccid in valid_iccds {
            assert!(Iccid::from_line(iccid).is_ok());
        }
    }
}
