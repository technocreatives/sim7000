use crate::modem::at_command::{ATParseErr, ATParseLine};

/// Voltage is out of range for the Sim7000
#[derive(Debug)]
pub enum VoltageWarning {
    OverVoltage,
    UnderVoltage,
}

impl ATParseLine for VoltageWarning {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let (reason, message) = line.split_once(' ').ok_or(ATParseErr)?;

        // looks like a typo in the documentation
        if !["WARNNING", "WARNING"].contains(&message) {
            return Err(ATParseErr);
        }

        match reason {
            "UNDER-VOLTAGE" => Ok(VoltageWarning::UnderVoltage),
            "OVER-VOLTAGE" => Ok(VoltageWarning::OverVoltage),
            _ => Err(ATParseErr),
        }
    }
}
