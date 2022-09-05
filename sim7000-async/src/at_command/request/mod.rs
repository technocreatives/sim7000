pub trait ATRequest {
    type Response;
    fn encode(&self) -> heapless::String<256>;
}

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
pub mod cpsi;
pub mod csclk;
pub mod csq;
pub mod cstt;
pub mod ifc;
pub mod ipr;

pub use at::At;
pub use ate::SetEcho;
pub use cbatchk::EnableVBatCheck;
pub use ccid::ShowIccid;
pub use cedrxs::{AcTType, ConfigureEDRX, EDRXSetting};
pub use cfgri::{ConfigureRiPin, RiPinMode};
pub use cgnspwr::SetGnssPower;
pub use cgnsurc::ConfigureGnssUrc;
pub use cgreg::{ConfigureRegistrationUrc, GetRegistrationStatus};
pub use cifsrex::GetLocalIpExt;
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
pub use cpsi::GetSystemInfo;
pub use csclk::SetSlowClock;
pub use csq::GetSignalQuality;
pub use cstt::StartTask;
pub use ifc::{FlowControl, SetFlowControl};
pub use ipr::{BaudRate, SetBaudRate};
