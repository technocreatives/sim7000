use core::{
    fmt::Debug,
    num::{ParseFloatError, ParseIntError},
};

pub mod generic_response;
pub mod unsolicited;

pub use generic_response::{CloseOk, GenericOk, SimError, WritePrompt};

pub mod at;
pub mod ate;
pub mod cbatchk;
pub mod ccid;
pub mod cedrxs;
pub mod cereg;
pub mod cfgri;
pub mod cgmr;
pub mod cgnapn;
pub mod cgnscold;
pub mod cgnscpy;
pub mod cgnsmod;
pub mod cgnspwr;
pub mod cgnsurc;
pub mod cgnsxtra;
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
pub mod cnact;
pub mod cnmp;
pub mod cntp;
pub mod cntpcid;
pub mod cops;
pub mod cpsi;
pub mod creg;
pub mod csclk;
pub mod csq;
pub mod cstt;
pub mod gsn;
pub mod httptofs;
pub mod ifc;
pub mod ipr;
pub mod sapbr;

pub use at::At;
pub use ate::SetEcho;
pub use cbatchk::EnableVBatCheck;
pub use ccid::{Iccid, ShowIccid};
pub use cedrxs::{AcTType, ConfigureEDRX, EDRXSetting};
pub use cfgri::{ConfigureRiPin, RiPinMode};
pub use cgmr::{FwVersion, GetFwVersion};
pub use cgnapn::{GetNetworkApn, NetworkApn};
pub use cgnscold::GnssColdStart;
pub use cgnscpy::CopyXtraFile;
pub use cgnsmod::{GetGnssWorkModeSet, SetGnssWorkModeSet};
pub use cgnspwr::SetGnssPower;
pub use cgnsurc::ConfigureGnssUrc;
pub use cgnsxtra::{GnssXtra, ToggleXtra};
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
pub use cnact::{CnactMode, SetAppNetwork};
pub use cnmp::{NetworkMode, SetNetworkMode};
pub use cntp::{Execute, SynchronizeNetworkTime};
pub use cntpcid::SetGprsBearerProfileId;
pub use cops::{GetOperatorInfo, OperatorFormat, OperatorInfo, OperatorMode};
pub use cpsi::{GetSystemInfo, SystemInfo, SystemMode};
pub use csclk::SetSlowClock;
pub use csq::{GetSignalQuality, SignalQuality};
pub use cstt::StartTask;
pub use gsn::{GetImei, Imei};
pub use httptofs::DownloadToFileSystem;
pub use ifc::{FlowControl, SetFlowControl};
pub use ipr::{BaudRate, SetBaudRate};
pub use sapbr::{BearerSettings, CmdType, ConParamType};

use self::{
    cgnscold::XtraStatus, cgnscpy::CopyResponse, cntp::NetworkTime, httptofs::DownloadInfo,
};

#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct AtParseErr {
    #[allow(dead_code)]
    message: &'static str,
}

pub(crate) trait AtParseLine: Sized {
    fn from_line(line: &str) -> Result<Self, AtParseErr>;
}

#[cfg(feature = "defmt")]
pub trait AtRequest: Debug + defmt::Format {
    type Response;
    fn encode(&self) -> heapless::String<256>;
}

#[cfg(not(feature = "defmt"))]
pub trait AtRequest: Debug {
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
    FwVersion(FwVersion),
    NetworkApn(NetworkApn),
    NetworkTime(NetworkTime),
    DownloadInfo(DownloadInfo),
    CopyResponse(CopyResponse),
    XtraStatus(XtraStatus),
    Imei(Imei),
}

impl AtParseLine for ResponseCode {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        /// Returns a function that tries to parse the line into a ResponseCode::T
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
            .or_else(parse(line, ResponseCode::FwVersion))
            .or_else(parse(line, ResponseCode::NetworkApn))
            .or_else(parse(line, ResponseCode::NetworkTime))
            .or_else(parse(line, ResponseCode::DownloadInfo))
            .or_else(parse(line, ResponseCode::CopyResponse))
            .or_else(parse(line, ResponseCode::XtraStatus))
            // Imei is weird and may not be unambiguously parsed.
            // Take care if trying to implement other, similar, response codes.
            .or_else(parse(line, ResponseCode::Imei))
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
