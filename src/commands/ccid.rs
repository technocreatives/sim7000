use embedded_time::duration::Milliseconds;

use crate::{Error, SerialReadTimeout};

use super::{AtCommand, AtDecode, AtExecute};

pub struct Ccid;

impl AtCommand for Ccid {
    const COMMAND: &'static str = "AT+CCID";
}

#[derive(Debug, Clone, Copy)]
pub struct Iccid {
    pub country: u8,
    pub issuer: u8,
    pub account: u64,
    // pub checksum: u8,
}

impl AtDecode for Iccid {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut super::Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        let string = decoder.remainder_str(timeout)?;

        //  "89 88 28 0666001104843 8"
        //  "89 01 26 0862291477114 f"
        if string.len() != 20 {
            return Err(crate::Error::DecodingFailed);
        }

        let _constant: u8 = string[0..2]
            .parse()
            .map_err(|_| crate::Error::DecodingFailed)?;
        let country = string[2..4]
            .parse()
            .map_err(|_| crate::Error::DecodingFailed)?;
        let issuer = string[4..6]
            .parse()
            .map_err(|_| crate::Error::DecodingFailed)?;
        let account = string[6..19]
            .parse()
            .map_err(|_| crate::Error::DecodingFailed)?;
        let _checksum: u8 =
            u8::from_str_radix(&string[19..20], 16).map_err(|_| crate::Error::DecodingFailed)?;

        let result = Iccid {
            country,
            issuer,
            account,
        };

        decoder.end_line();
        decoder.expect_str("OK", timeout)?;

        Ok(result)
    }
}

impl AtExecute for Ccid {
    type Output = Iccid;
}
