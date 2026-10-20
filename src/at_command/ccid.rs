use heapless::String;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

/// AT+CCID
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
        // ICCIDs are 19 or 20 digits, a 19 digit ICCID is padded with an 'f' by the modem.
        let line = line.trim_end_matches(['f', 'F']);
        if !(19..=20).contains(&line.len()) {
            return Err("Invalid length".into());
        }

        if &line[..2] != "89" {
            return Err("Invalid MII".into());
        }

        let country = line[2..4].parse()?;
        let issuer = line[4..6].parse()?;
        let account = line[6..].parse()?;

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
        let valid_iccds = [
            "89882806660011048438",
            "8901260862291477114f",
            "8901260862291477114F",
        ];

        for iccid in valid_iccds {
            assert!(Iccid::from_line(iccid).is_ok());
        }
    }

    #[test]
    fn parse_iccid_20_digits() {
        let iccid = Iccid::from_line("89882806660011048438").unwrap();
        assert_eq!(iccid.country, 88);
        assert_eq!(iccid.issuer, 28);
        assert_eq!(iccid.account, 6660011048438);
    }

    #[test]
    fn parse_iccid_19_digits() {
        let iccid = Iccid::from_line("8901260862291477114f").unwrap();
        assert_eq!(iccid.country, 1);
        assert_eq!(iccid.issuer, 26);
        assert_eq!(iccid.account, 862291477114);
    }

    #[test]
    fn parse_iccid_invalid() {
        let invalid_iccids = [
            "898828066600110484",    // 18 digits
            "898828066600110484381", // 21 digits
            "79882806660011048438",  // wrong MII
            "89882806660011048f38",  // padding in the middle
        ];

        for iccid in invalid_iccids {
            assert!(Iccid::from_line(iccid).is_err(), "{iccid}");
        }
    }
}
