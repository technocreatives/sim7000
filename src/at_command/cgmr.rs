use core::str::FromStr;

use heapless::String;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

/// Maximum version length that we can read.
///
/// The version of the modem I'm currently testing with is "1529B07SIM7000G", so
/// 32 is probably fine.
const MAX_VERSION_LEN: usize = 32;

/// AT+CGMR
///
/// Gets the "product software version identification text", i.e. some kind of
/// identifier for the version of the firmware running on the modem.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GetFwVersion;

impl AtRequest for GetFwVersion {
    type Response = (FwVersion, GenericOk);

    fn encode(&self) -> String<256> {
        "AT+CGMR\r".into()
    }
}

/// Firmware revision of a modem.
///
/// This is just a newtype wrapper around a `heapless::String` since the format
/// of the version isn't documented. However, the format *seems* to be something
/// like the following:
///
/// ```text
/// 1529B07SIM7000G
/// └┬─┘│├┘└──┬───┘
///  │  ││    └── The model number of the modem.
///  │  ││
///  │  │└── A revision number. This seems to be incremented by 1 for every new
///  │  │    version of the firmware.
///  │  │
///  │  └── A "B", indicating that the following number is a build number?
///  │
///  └── Some kind of hardware version number. This doesn't seem to change
///      between firmware versions.
/// ```
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FwVersion(pub String<MAX_VERSION_LEN>);

impl AtParseLine for FwVersion {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let version = line
            .strip_prefix("Revision:")
            .ok_or(AtParseErr::from("Line does not start with \"Revision:\""))?;

        String::from_str(version)
            .map(Self)
            .map_err(|_| AtParseErr::from("Modem firmware version is too long"))
    }
}

impl AtResponse for FwVersion {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::FwVersion(fw_version) => Ok(fw_version),
            _ => Err(code),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_version() {
        assert!(FwVersion::from_line("Revision:1529B07SIM7000G").is_ok());
        assert!(FwVersion::from_line("complete bogus").is_err());
    }
}
