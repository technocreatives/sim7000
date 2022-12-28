use heapless::String;

use crate::util::collect_array;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

/// AT+COPS?
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GetOperatorInfo;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OperatorInfo {
    pub mode: OperatorMode,
    pub format: OperatorFormat,
    pub operator_name: heapless::String<256>,
}

#[repr(u8)]
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum OperatorMode {
    Automatic = 0,
    Manual = 1,
    ManualDeregister = 2,
    ManualAutomatic = 4,
}

#[repr(u8)]
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum OperatorFormat {
    Long = 0,
    Short = 1,
    Numeric = 2,
}

impl AtRequest for GetOperatorInfo {
    type Response = (OperatorInfo, GenericOk);
    fn encode(&self) -> String<256> {
        "AT+COPS?\r".into()
    }
}

impl AtParseLine for OperatorInfo {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        // +COPS: 0,0,\"Tele2 1nce.net\",7"

        let line = line.strip_prefix("+COPS: ").ok_or("Missing '+COPS '")?;

        let [mode, format, operator_name, _netact] =
            collect_array(line.splitn(4, ',')).ok_or("Missing ','")?;

        let mode = match mode {
            "0" => OperatorMode::Automatic,
            "1" => OperatorMode::Manual,
            "2" => OperatorMode::ManualDeregister,
            "4" => OperatorMode::ManualAutomatic,
            _ => return Err("Failed to parse mode".into()),
        };

        let format = match format {
            "0" => OperatorFormat::Long,
            "1" => OperatorFormat::Short,
            "2" => OperatorFormat::Numeric,
            _ => return Err("Failed to parse format".into()),
        };

        let operator_name = operator_name.trim_matches('"').into();

        Ok(OperatorInfo {
            mode,
            format,
            operator_name,
        })
    }
}

impl AtResponse for OperatorInfo {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::OperatorInfo(info) => Ok(info),
            _ => Err(code),
        }
    }
}
