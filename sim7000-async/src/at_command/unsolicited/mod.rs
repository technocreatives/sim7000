//! Unsolicited Response Codes

use super::{AtParseErr, AtParseLine};

mod app_pdp;
mod cfun;
mod cgreg;
mod connection;
mod cpin;
mod creg;
mod cring;
mod ctzv;
mod cusd;
mod dst;
mod pdp;
mod power_down;
mod psnwid;
mod psuttz;
mod rdy;
mod receive;
mod remote_ip;
mod sms_ready;
mod ugnsinf;
mod voltage_warning;

pub use app_pdp::AppNetworkActive;
pub use cfun::CFun;
pub use cgreg::RegistrationStatus;
pub use connection::{Connection, ConnectionMessage};
pub use cpin::CPin;
pub use creg::CReg;
pub use cring::CRing;
pub use ctzv::Ctzv;
pub use cusd::CUsd;
pub use dst::Dst;
pub use pdp::GprsDisconnected;
pub use power_down::PowerDown;
pub use psnwid::Pdnwid;
pub use psuttz::Psuttz;
pub use rdy::Ready;
pub use receive::ReceiveHeader;
pub use remote_ip::IncomingConnection;
pub use sms_ready::SmsReady;
pub use ugnsinf::{GnssFix, GnssReport};
pub use voltage_warning::VoltageWarning;

/// Unsolicited Response Code
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Urc {
    AppNetworkActive(AppNetworkActive),
    CFun(CFun),
    CPin(CPin),
    CReg(CReg),
    CRing(CRing),
    CUsd(CUsd),
    ConnectionMessage(Connection),
    Ctzv(Ctzv),
    Dst(Dst),
    GnssReport(GnssReport),
    GprsDisconnected(GprsDisconnected),
    Pdnwid(Pdnwid),
    PowerDown(PowerDown),
    Psuttz(Psuttz),
    Ready(Ready),
    SmsReady(SmsReady),
    ReceiveHeader(ReceiveHeader),
    RegistrationStatus(RegistrationStatus),
    VoltageWarning(VoltageWarning),
}

impl AtParseLine for Urc {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        /// Returns a function that tries to parse the line into a Urc::T
        fn parse<'a, T: AtParseLine>(
            line: &'a str,
            f: impl Fn(T) -> Urc + 'a,
        ) -> impl Fn(AtParseErr) -> Result<Urc, AtParseErr> + 'a {
            move |_| Ok(f(T::from_line(line)?))
        }

        Err(AtParseErr::default())
            .or_else(parse(line, Urc::AppNetworkActive))
            .or_else(parse(line, Urc::CFun))
            .or_else(parse(line, Urc::CPin))
            .or_else(parse(line, Urc::CReg))
            .or_else(parse(line, Urc::CRing))
            .or_else(parse(line, Urc::CUsd))
            .or_else(parse(line, Urc::ConnectionMessage))
            .or_else(parse(line, Urc::Ctzv))
            .or_else(parse(line, Urc::Dst))
            .or_else(parse(line, Urc::GnssReport))
            .or_else(parse(line, Urc::GprsDisconnected))
            .or_else(parse(line, Urc::Pdnwid))
            .or_else(parse(line, Urc::PowerDown))
            .or_else(parse(line, Urc::Psuttz))
            .or_else(parse(line, Urc::Ready))
            .or_else(parse(line, Urc::SmsReady))
            .or_else(parse(line, Urc::ReceiveHeader))
            .or_else(parse(line, Urc::RegistrationStatus))
            .or_else(parse(line, Urc::VoltageWarning))
            .map_err(|_| AtParseErr::from("Failed to parse as a URC"))
    }
}

// TODO
//mod cdnsgip
//mod cmti;
//mod cmt;
//mod cbm;
//mod cds;
