use super::{AtCommand, AtDecode, AtExecute, Decoder};
use crate::{Error, SerialReadTimeout};

pub struct Cipstatus;

impl AtCommand for Cipstatus {
    const COMMAND: &'static str = "AT+CIPSTATUS";
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ConnectionState {
    IpInitial,
    IpStart,
    IpConfig,
    IpGprsact,
    IpStatus,
    TcpConnecting,
    UdpConnecting,
    ServerListening,
    ConnectOk,
    TcpClosing,
    UdpClosing,
    TcpClosed,
    UdpClosed,
    PdpDeact,
}

impl TryFrom<&str> for ConnectionState {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "IP INITIAL" => ConnectionState::IpInitial,
            "IP START" => ConnectionState::IpStart,
            "IP CONFIG" => ConnectionState::IpConfig,
            "IP GPRSACT" => ConnectionState::IpGprsact,
            "IP STATUS" => ConnectionState::IpStatus,
            "TCP CONNECTING" => ConnectionState::TcpConnecting,
            "UDP CONNECTING" => ConnectionState::UdpConnecting,
            "SERVER LISTENING" => ConnectionState::ServerListening,
            "CONNECT OK" => ConnectionState::ConnectOk,
            "TCP CLOSING" => ConnectionState::TcpClosing,
            "UDP CLOSING" => ConnectionState::UdpClosing,
            "TCP CLOSED" => ConnectionState::TcpClosing,
            "UDP CLOSED" => ConnectionState::UdpClosed,
            "PDP DEACT" => ConnectionState::PdpDeact,
            _ => return Err(()),
        })
    }
}

impl AtDecode for ConnectionState {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout_ms: u32,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("OK", timeout_ms)?;
        decoder.end_line();

        decoder.expect_str("STATE: ", timeout_ms)?;
        let state = ConnectionState::try_from(decoder.remainder_str(timeout_ms)?)
            .map_err(|_| crate::Error::DecodingFailed)?;

        Ok(state)
    }
}

impl AtExecute for Cipstatus {
    type Output = ConnectionState;
}
