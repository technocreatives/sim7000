use heapless::String;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

/// AT+CGNAPN
///
/// Get Network APN in CatM or NbIot
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GetNetworkApn;

/// Response to [GetNetworkApn].
///
/// `apn` will be [None] if the network did not send us an APN,
/// or if we're not in CatM or NbIot mode.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NetworkApn {
    // The maximum length of an APN is 63 octets (bytes)
    pub apn: Option<String<63>>,
}

impl AtRequest for GetNetworkApn {
    type Response = (NetworkApn, GenericOk);
    fn encode(&self) -> String<256> {
        "AT+CGNAPN\r".into()
    }
}

impl AtParseLine for NetworkApn {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let line = line
            .strip_prefix("+CGNAPN: ")
            .ok_or("Missing '+CGNAPN: '")?;

        let (valid, apn) = line.split_once(',').ok_or("Missing ','")?;

        match valid {
            "0" => Ok(NetworkApn { apn: None }),
            "1" => {
                let apn = apn.trim_matches('"');
                #[allow(clippy::unnecessary_fallible_conversions)] // heapless string panics on from
                let apn = String::try_from(apn).map_err(|_| "APN too long")?;
                Ok(NetworkApn { apn: Some(apn) })
            }
            _ => Err("Invalid 'valid' field, expected 1 or 0".into()),
        }
    }
}

impl AtResponse for NetworkApn {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::NetworkApn(v) => Ok(v),
            _ => Err(code),
        }
    }
}
