use embedded_time::duration::Milliseconds;

use crate::{Error, SerialReadTimeout};

use super::{AtCommand, AtDecode, AtRead, Decoder};

pub struct Cpsi;

impl AtCommand for Cpsi {
    const COMMAND: &'static str = "AT+CPSI";
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SystemMode {
    NoService,
    Gsm,
    LteCatM1,
    LteNbIot,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OperationMode {
    Online,
    Offline,
    FactoryTest,
    Reset,
    LowPower,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

        // The SIM7000 may respond with an extra empty line in GSM mode for no reason
        match decoder.remainder_str(timeout)? {
            "OK" => {
                decoder.expect_str("OK", timeout)?;
                return Ok(SystemInfo {
                    system_mode,
                    operation_mode,
                });
            }
            "" => {
                decoder.end_line();
            }
            _ => return Err(Error::DecodingFailed),
        }

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

#[cfg(test)]
mod test {
    use embedded_time::duration::Milliseconds;

    use crate::{commands::AtRead, test::MockSerial};

    use super::{Cpsi, OperationMode, SystemInfo, SystemMode};

    #[test]
    fn test_gsm_response() {
        let mut mock = MockSerial::build()
            .expect_write(b"AT+CPSI?\r")
            .expect_read(b"\r\n+CPSI: GSM,Online,240-01,0x11a4,25882,80 EGSM 900,-89,0,22-22\r\n")
            .expect_read(b"\r\n\r\n")
            .expect_read(b"\r\nOK\r\n")
            .finalize();

        let response = Cpsi.read(&mut mock, Milliseconds(1000)).unwrap();

        assert_eq!(
            response,
            SystemInfo {
                system_mode: SystemMode::Gsm,
                operation_mode: OperationMode::Online
            }
        )
    }

    #[test]
    fn test_lte_response() {
        let mut mock = MockSerial::build()
            .expect_write(b"AT+CPSI?\r")
            .expect_read(b"\r\n+CPSI: LTE CAT-M1,Online,240-01,0x0081,25716767,254,EUTRAN-BAND3,1300,5,5,-13,-84,-54,18\r\n")
            .expect_read(b"\r\nOK\r\n")
            .finalize();

        let response = Cpsi.read(&mut mock, Milliseconds(1000)).unwrap();

        assert_eq!(
            response,
            SystemInfo {
                system_mode: SystemMode::LteCatM1,
                operation_mode: OperationMode::Online
            }
        )
    }
}
