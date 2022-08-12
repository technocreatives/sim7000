use crate::modem::at_command::{ATParseErr, ATParseLine};

/// TCP Connection result
pub struct ConnectionResult {
    channel: usize,
    result: ConnectionResultKind,
}

pub enum ConnectionResultKind {
    Ok,
    Fail,
    AlreadyConnected,
}

impl ATParseLine for ConnectionResult {
    fn parse_from_line(line: &str) -> Result<Self, ATParseErr> {}
}
