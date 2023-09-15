use heapless::String;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

/// AT+GSN
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GetImei;

impl AtRequest for GetImei {
    type Response = (Imei, GenericOk);
    fn encode(&self) -> String<256> {
        "AT+GSN\r".into()
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Imei {
    pub imei: String<16>,
}

impl AtParseLine for Imei {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        if ![15, 16].contains(&line.len()) {
            return Err("Invalid length".into());
        }

        if line.chars().any(|c| !c.is_digit(10)) {
            return Err("Contains non-digit character".into());
        }

        Ok(Imei {
            imei: line.into(),
        })
    }
}

impl AtResponse for Imei {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::Imei(v) => Ok(v),
            _ => Err(code),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn parse_cpsi() {
        let valid_imeis = [
            "49015420323751",
        ];

        for valid in valid_imeis {
            assert!(Imei::from_line(valid).is_ok());
        }
    }
}
