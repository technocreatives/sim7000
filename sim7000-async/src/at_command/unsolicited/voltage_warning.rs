use crate::at_command::{ATParseErr, ATParseLine};

/// Voltage is out of range for the Sim7000
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum VoltageWarning {
    OverVoltage,
    UnderVoltage,
}

impl ATParseLine for VoltageWarning {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let (reason, message) = line.split_once(' ').ok_or("Missing ' '")?;

        // looks like a typo in the documentation
        if !["WARNNING", "WARNING"].contains(&message) {
            return Err("Missing 'WARNING'".into());
        }

        match reason {
            "UNDER-VOLTAGE" => Ok(VoltageWarning::UnderVoltage),
            "OVER-VOLTAGE" => Ok(VoltageWarning::OverVoltage),
            _ => Err("Invalid reason, expected OVER or UNDER-VOLTAGE".into()),
        }
    }
}
