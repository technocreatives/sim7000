use crate::at_command::{AtParseErr, AtParseLine};

/// Indicates whether the app network is active
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AppNetworkActive(pub bool);

impl AtParseLine for AppNetworkActive {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        match line {
            "+APP PDP: ACTIVE" => Ok(AppNetworkActive(true)),
            "+APP PDP: DEACTIVE" => Ok(AppNetworkActive(false)),
            _ => Err("Expecting '+APP PDP: ACTIVE/DEACTIVE'".into()),
        }
    }
}
