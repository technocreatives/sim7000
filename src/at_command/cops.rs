use core::fmt::Write;
use heapless::String;

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

/// AT+COPS=1,2,"<plmn>"
///
/// Manually select an operator by its numeric PLMN (MCC + MNC, e.g. "64304").
/// The modem will not fall back to automatic selection if the operator is unavailable.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetManualOperator {
    pub plmn: String<6>,
}

impl AtRequest for SetManualOperator {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(
            buf,
            "AT+COPS={},{},\"{}\"\r",
            OperatorMode::Manual as u8,
            OperatorFormat::Numeric as u8,
            self.plmn
        )
        .unwrap();
        buf
    }
}

/// AT+COPS=0
///
/// Let the modem select the operator automatically.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetAutomaticOperator;

impl AtRequest for SetAutomaticOperator {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        "AT+COPS=0\r".into()
    }
}

impl AtParseLine for OperatorInfo {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        // +COPS: 0,0,\"Tele2 1nce.net\",7"
        // The format and operator are omitted when no operator is selected, e.g. "+COPS: 0"

        let line = line.strip_prefix("+COPS: ").ok_or("Missing '+COPS '")?;

        let mut fields = line.splitn(4, ',');
        let mode = fields.next().ok_or("Missing mode")?;
        let format = fields.next().unwrap_or("0");
        let operator_name = fields.next().unwrap_or("");

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encode_set_manual_operator() {
        let request = SetManualOperator {
            plmn: "64304".into(),
        };
        assert_eq!(request.encode().as_str(), "AT+COPS=1,2,\"64304\"\r");
    }

    #[test]
    fn parse_operator_info() {
        let info = OperatorInfo::from_line("+COPS: 0,0,\"Tele2 1nce.net\",7").unwrap();
        assert!(matches!(info.mode, OperatorMode::Automatic));
        assert!(matches!(info.format, OperatorFormat::Long));
        assert_eq!(info.operator_name.as_str(), "Tele2 1nce.net");
    }

    #[test]
    fn parse_operator_info_without_operator() {
        let info = OperatorInfo::from_line("+COPS: 1").unwrap();
        assert!(matches!(info.mode, OperatorMode::Manual));
        assert_eq!(info.operator_name.as_str(), "");
    }

    #[test]
    fn parse_operator_info_without_netact() {
        let info = OperatorInfo::from_line("+COPS: 1,2,\"64304\"").unwrap();
        assert!(matches!(info.mode, OperatorMode::Manual));
        assert!(matches!(info.format, OperatorFormat::Numeric));
        assert_eq!(info.operator_name.as_str(), "64304");
    }
}
