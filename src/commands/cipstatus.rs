use super::{AtCommand, AtDecode, AtExecute, Decoder};
use crate::{Error, SerialReadTimeout};
use embedded_time::duration::Milliseconds;

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

impl AtDecode for ConnectionState {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("OK", timeout)?;
        decoder.end_line();
        decoder.expect_empty(timeout)?;
        decoder.end_line();

        decoder.expect_str("STATE: ", timeout)?;
        let state = match decoder.remainder_str(timeout)? {
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
            _ => return Err(crate::Error::DecodingFailed),
        };

        Ok(state)
    }
}

impl AtExecute for Cipstatus {
    type Output = ConnectionState;
}
