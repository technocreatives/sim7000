use crate::modem::at_command::{ATParseErr, ATParseLine};

/// Indicates whether the app network is active
#[derive(Debug)]
pub struct AppNetworkActive(pub bool);

impl ATParseLine for AppNetworkActive {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        match line {
            "+APP PDP: ACTIVE" => Ok(AppNetworkActive(true)),
            "+APP PDP: DEACTIVE" => Ok(AppNetworkActive(false)),
            _ => Err(ATParseErr),
        }
    }
}
