use embedded_time::duration::Milliseconds;

use crate::{Error, SerialReadTimeout, SerialWrite};

use super::{AtCommand, AtDecode, AtEncode, AtRead, AtWrite, Decoder, Encoder};

pub struct Cgreg;

impl AtCommand for Cgreg {
    const COMMAND: &'static str = "AT+CGREG";
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RegistrationStatus {
    NotRegistered,
    RegisteredHome,
    Searching,
    RegistrationDenied,
    Unknown,
    RegisteredRoaming,
}

#[repr(u8)]
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RegistrationMode {
    Disable = 0,
    EnableReg = 1,
    EnableRegLac = 2,
    EnableRegLacTime = 4,
}

#[derive(Copy, Clone)]
pub struct RegistrationResponse {
    pub mode: RegistrationMode,
    pub stat: RegistrationStatus,
}

impl AtDecode for RegistrationResponse {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("+CGREG: ", timeout)?;

        let mode = match decoder.decode_scalar(timeout)? {
            0 => RegistrationMode::Disable,
            1 => RegistrationMode::EnableReg,
            2 => RegistrationMode::EnableRegLac,
            4 => RegistrationMode::EnableRegLacTime,
            _ => return Err(crate::Error::DecodingFailed),
        };

        decoder.expect_str(",", timeout)?;
        let stat = match decoder.decode_scalar(timeout)? {
            0 => RegistrationStatus::NotRegistered,
            1 => RegistrationStatus::RegisteredHome,
            2 => RegistrationStatus::Searching,
            3 => RegistrationStatus::RegistrationDenied,
            4 => RegistrationStatus::Unknown,
            5 => RegistrationStatus::RegisteredRoaming,
            _ => return Err(crate::Error::DecodingFailed),
        };

        decoder.end_line();
        decoder.expect_empty(timeout)?;
        decoder.end_line();
        decoder.expect_str("OK", timeout)?;

        Ok(RegistrationResponse { mode, stat })
    }
}

impl AtEncode for RegistrationMode {
    fn encode<B: SerialWrite>(
        &self,
        encoder: &mut Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self as i32)
    }
}

impl AtRead for Cgreg {
    type Output = RegistrationResponse;
}

impl AtWrite<'_> for Cgreg {
    type Input = RegistrationMode;
    type Output = ();
}
