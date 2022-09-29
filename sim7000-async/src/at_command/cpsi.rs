use heapless::String;

use crate::util::collect_array;

use super::{AtParseErr, AtParseLine, AtRequest, AtResponse, GenericOk, ResponseCode};

pub struct GetSystemInfo;

impl AtRequest for GetSystemInfo {
    type Response = (SystemInfo, GenericOk);
    fn encode(&self) -> String<256> {
        "AT+CPSI?\r".into()
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SystemMode {
    NoService,
    Gsm,
    LteCatM1,
    LteNbIot,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum OperationMode {
    Online,
    Offline,
    FactoryTest,
    Reset,
    LowPower,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SystemInfo {
    pub system_mode: SystemMode,
    pub operation_mode: OperationMode,
}

impl AtParseLine for SystemInfo {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let line = line.strip_prefix("+CPSI: ").ok_or("Missing '+CPSI: '")?;
        let [system_mode, operation_mode, _mcc, _mnc, _lac, _cell_id, _absolute_rf_ch_num, _rx_lev, _track_lo_adjust, _c1_c2] =
            collect_array(line.splitn(10, ',')).ok_or("Missing ','")?;

        let system_mode = match system_mode {
            "NO SERVICE" => SystemMode::NoService,
            "GSM" => SystemMode::Gsm,
            "LTE CAT-M1" => SystemMode::LteCatM1,
            "LTE NB-IOT" => SystemMode::LteNbIot,
            _ => return Err("Failed to parse System Mode".into()),
        };

        let operation_mode = match operation_mode {
            "Online" => OperationMode::Online,
            "Offline" => OperationMode::Offline,
            "Factory Test Mode" => OperationMode::FactoryTest,
            "Reset" => OperationMode::Reset,
            "Low Power Mode" => OperationMode::LowPower,
            _ => return Err("Failed to parse Operation Mode".into()),
        };

        Ok(SystemInfo {
            system_mode,
            operation_mode,
        })
    }
}

impl AtResponse for SystemInfo {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::SystemInfo(v) => Ok(v),
            _ => Err(code),
        }
    }
}
