use embedded_time::duration::Milliseconds;

use crate::{Error, SerialReadTimeout};

use super::{AtCommand, AtDecode, AtRead, Decoder};

pub struct Cpsi;

impl AtCommand for Cpsi {
    const COMMAND: &'static str = "AT+CPSI";
}

#[derive(Copy, Clone, Debug)]
pub enum SystemMode {
    NoService,
    Gsm,
    LteCatM1,
    LteNbIot,
}

#[derive(Copy, Clone, Debug)]
pub enum OperationMode {
    Online,
    Offline,
    FactoryTest,
    Reset,
    LowPower,
}

#[derive(Copy, Clone)]
pub struct SystemInfo {
    pub system_mode: SystemMode,
    pub operation_mode: OperationMode,
}

impl AtDecode for SystemInfo {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("+CPSI: ", timeout)?;

        let mut components = decoder.remainder_str(timeout)?.split(',');

        let system_mode = match components.next().ok_or(crate::Error::DecodingFailed)? {
            "NO SERVICE" => SystemMode::NoService,
            "GSM" => SystemMode::Gsm,
            "LTE CAT-M1" => SystemMode::LteCatM1,
            "LTE NB-IOT" => SystemMode::LteNbIot,
            _ => return Err(crate::Error::DecodingFailed),
        };

        let operation_mode = match components.next().ok_or(crate::Error::DecodingFailed)? {
            "Online" => OperationMode::Online,
            "Offline" => OperationMode::Offline,
            "Factory Test Mode" => OperationMode::FactoryTest,
            "Reset" => OperationMode::Reset,
            "Low Power Mode" => OperationMode::LowPower,
            _ => return Err(crate::Error::DecodingFailed),
        };

        decoder.end_line();
        decoder.expect_empty(timeout)?;
        decoder.end_line();
        decoder.expect_str("OK", timeout)?;

        Ok(SystemInfo {
            system_mode,
            operation_mode,
        })
    }
}

impl AtRead for Cpsi {
    type Output = SystemInfo;
}
