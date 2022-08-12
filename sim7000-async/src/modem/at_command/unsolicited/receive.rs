use crate::modem::at_command::{ATParseErr, ATParseLine};

/// The modem is receiving data on a connection. It will transmit `length` bytes right after this header.
#[derive(Debug)]
pub struct ReceiveHeader {
    pub connection: usize,
    pub length: usize,
}

impl ATParseLine for ReceiveHeader {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let (message, rest) = line.split_once(',').ok_or(ATParseErr)?;

        if message != "+RECEIVE" {
            return Err(ATParseErr);
        }

        let (connection, length) = rest
            .trim_end_matches(':')
            .split_once(',')
            .ok_or(ATParseErr)?;

        Ok(ReceiveHeader {
            connection: connection.parse()?,
            length: length.parse()?,
        })
    }
}
