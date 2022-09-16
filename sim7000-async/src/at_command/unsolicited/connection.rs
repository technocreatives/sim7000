use crate::at_command::{ATParseErr, ATParseLine};

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Connection {
    pub index: usize,
    pub message: ConnectionMessage,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConnectionMessage {
    /// The connection was successfully established
    Connected,

    /// Failed to establish connection
    ConnectionFailed,

    /// A connection already exists on this index
    AlreadyConnected,

    /// A message was successfully sent
    SendSuccess,

    /// Failed to send message
    SendFail,

    /// The connection was closed
    Closed,
}

impl ATParseLine for Connection {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let (index, message) = line.split_once(", ").ok_or("Missing ', '")?;
        let index = index.parse()?;

        use ConnectionMessage::*;
        let message = match message {
            "CLOSED" => Closed,
            "SEND OK" => SendSuccess,
            "SEND FAIL" => SendFail,
            "CONNECT OK" => Connected,
            "CONNECT FAIL" => ConnectionFailed,
            "ALREADY CONNECT" => AlreadyConnected,
            _ => {
                return Err("Invalid connection message".into());
            }
        };

        Ok(Connection { index, message })
    }
}
