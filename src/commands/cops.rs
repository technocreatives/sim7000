use embedded_time::duration::Milliseconds;

use crate::{Error, SerialReadTimeout};

use super::{AtCommand, AtDecode, AtRead, Decoder};

pub struct Cops;

impl AtCommand for Cops {
    const COMMAND: &'static str = "AT+COPS";
}

#[derive(Copy, Clone, Debug)]
pub enum Mode {
    Automatic,
    Manual,
    ManualDeregister,
    ManualAutomatic,
}

#[derive(Copy, Clone, Debug)]
pub enum Format {
    Long,
    Short,
    Numeric,
}

#[derive(Clone, Debug)]
pub struct OperatorInfo {
    pub mode: Mode,
    pub format: Format,
    pub operator_name: heapless::String<256>,
}

impl AtDecode for OperatorInfo {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("+COPS: ", timeout)?;
        let mut components = decoder.remainder_str(timeout)?.split(',');

        let mode = match components
            .next()
            .and_then(|v| v.parse().ok())
            .ok_or(crate::Error::DecodingFailed)?
        {
            0 => Mode::Automatic,
            1 => Mode::Manual,
            2 => Mode::ManualDeregister,
            4 => Mode::ManualAutomatic,
            _ => return Err(crate::Error::DecodingFailed),
        };

        let format = match components
            .next()
            .and_then(|v| v.parse().ok())
            .ok_or(crate::Error::DecodingFailed)?
        {
            0 => Format::Long,
            1 => Format::Short,
            2 => Format::Numeric,
            _ => return Err(crate::Error::DecodingFailed),
        };

        let operator_name = components.next().ok_or(crate::Error::DecodingFailed)?;
        let stripped_name = operator_name
            .strip_suffix('"')
            .unwrap_or(operator_name)
            .strip_prefix('"')
            .unwrap_or(operator_name)
            .into();

        decoder.end_line();
        decoder.expect_str("OK", timeout)?;

        Ok(OperatorInfo {
            mode,
            format,
            operator_name: stripped_name,
        })
    }
}

impl AtRead for Cops {
    type Output = OperatorInfo;
}
