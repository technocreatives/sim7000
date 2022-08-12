use crate::modem::at_command::{ATParseErr, ATParseLine};

// stub type
/// Network time zone
#[derive(Debug)]
pub enum PowerDown {
    /// Normal power down, triggered by a command or by the power pin.
    Normal,

    /// Chip automatically powered down due to under-voltage
    UnderVoltage,

    /// Chip automatically powered down due to over-voltage
    OverVoltage,
}

impl ATParseLine for PowerDown {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        // example: `NORMAL POWER DOWN`
        let (reason, message) = line.split_once(' ').ok_or(ATParseErr)?;

        if message != "POWER DOWN" {
            return Err(ATParseErr);
        }

        match reason {
            "NORMAL" => Ok(PowerDown::Normal),
            "UNDER-VOLTAGE" => Ok(PowerDown::UnderVoltage),
            "OVER-VOLTAGE" => Ok(PowerDown::OverVoltage),
            _ => Err(ATParseErr),
        }
    }
}
