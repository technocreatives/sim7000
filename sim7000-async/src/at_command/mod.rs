use core::num::{ParseFloatError, ParseIntError};

pub mod generic_response;
pub mod unsolicited;

pub use generic_response::{CloseOk, GenericOk, SimError, WritePrompt};

pub mod at;
pub mod ate;
pub mod cbatchk;
pub mod ccid;
pub mod cedrxs;
pub mod cfgri;
pub mod cgnspwr;
pub mod cgnsurc;
pub mod cgreg;
pub mod cifsrex;
pub mod ciicr;
pub mod cipclose;
pub mod cipmux;
pub mod cipsend;
pub mod cipshut;
pub mod cipsprt;
pub mod cipstart;
pub mod cmee;
pub mod cmnb;
pub mod cnmp;
pub mod cops;
pub mod cpsi;
pub mod csclk;
pub mod csq;
pub mod cstt;
pub mod ifc;
pub mod ipr;

pub use at::At;
pub use ate::SetEcho;
pub use cbatchk::EnableVBatCheck;
pub use ccid::{Iccid, ShowIccid};
pub use cedrxs::{AcTType, ConfigureEDRX, EDRXSetting};
pub use cfgri::{ConfigureRiPin, RiPinMode};
pub use cgnspwr::SetGnssPower;
pub use cgnsurc::ConfigureGnssUrc;
pub use cgreg::{ConfigureRegistrationUrc, GetRegistrationStatus};
pub use cifsrex::{GetLocalIpExt, IpExt};
pub use ciicr::StartGprs;
pub use cipclose::CloseConnection;
pub use cipmux::EnableMultiIpConnection;
pub use cipsend::IpSend;
pub use cipshut::ShutConnections;
pub use cipsprt::SetCipSendPrompt;
pub use cipstart::{Connect, ConnectMode};
pub use cmee::{CMEErrorMode, ConfigureCMEErrors};
pub use cmnb::{NbMode, SetNbMode};
pub use cnmp::{NetworkMode, SetNetworkMode};
pub use cops::{GetOperatorInfo, OperatorFormat, OperatorInfo, OperatorMode};
pub use cpsi::{GetSystemInfo, SystemInfo, SystemMode};
pub use csclk::SetSlowClock;
pub use csq::{GetSignalQuality, SignalQuality};
pub use cstt::StartTask;
pub use ifc::{FlowControl, SetFlowControl};
pub use ipr::{BaudRate, SetBaudRate};

#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct AtParseErr {
    #[allow(dead_code)]
    message: &'static str,
}

pub(crate) trait AtParseLine: Sized {
    fn from_line(line: &str) -> Result<Self, AtParseErr>;
}

pub trait AtRequest {
    type Response;
    fn encode(&self) -> heapless::String<256>;
}

pub trait AtResponse: Sized {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode>;
}

/// Sim7000 AT-command response code
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ResponseCode {
    Ok(GenericOk),
    Error(SimError),
    WritePrompt(WritePrompt), // "> "
    CloseOk(CloseOk),
    IpExt(IpExt),
    Iccid(Iccid),
    SignalQuality(SignalQuality),
    SystemInfo(SystemInfo),
    OperatorInfo(OperatorInfo),
}

impl AtParseLine for ResponseCode {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        /// Create a function that tries to parse the line into an Urc::T
        fn parse<'a, T: AtParseLine>(
            line: &'a str,
            f: impl Fn(T) -> ResponseCode + 'a,
        ) -> impl Fn(AtParseErr) -> Result<ResponseCode, AtParseErr> + 'a {
            move |_| Ok(f(T::from_line(line)?))
        }

        Err(AtParseErr::default())
            .or_else(parse(line, ResponseCode::Ok))
            .or_else(parse(line, ResponseCode::Error))
            .or_else(parse(line, ResponseCode::WritePrompt))
            .or_else(parse(line, ResponseCode::CloseOk))
            .or_else(parse(line, ResponseCode::IpExt))
            .or_else(parse(line, ResponseCode::Iccid))
            .or_else(parse(line, ResponseCode::SignalQuality))
            .or_else(parse(line, ResponseCode::SystemInfo))
            .or_else(parse(line, ResponseCode::OperatorInfo))
            .map_err(|_| "Unknown response code".into())
    }
}

impl From<&'static str> for AtParseErr {
    fn from(message: &'static str) -> Self {
        AtParseErr { message }
    }
}

impl From<ParseIntError> for AtParseErr {
    fn from(_: ParseIntError) -> Self {
        AtParseErr {
            message: "Failed to parse integer",
        }
    }
}

impl From<ParseFloatError> for AtParseErr {
    fn from(_: ParseFloatError) -> Self {
        AtParseErr {
            message: "Failed to parse float",
        }
    }
}

/// Stub AT response parser that just checks if the line starts with `prefix`
fn stub_parser_prefix<T>(line: &str, prefix: &'static str, t: T) -> Result<T, AtParseErr> {
    line.starts_with(prefix).then(|| t).ok_or(AtParseErr {
        message: "Stub parser: Missing prefix",
    })
}
