use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use futures::{select_biased, FutureExt};

use crate::at_command::unsolicited::{GnssFix, GnssReport};
use crate::drop::{AsyncDrop, DropChannel, DropMessage};
use crate::modem::power::PowerSignalListener;
use crate::{log, PowerState};

pub const GNSS_SLOTS: usize = 1;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Closed;

pub struct Gnss<'c> {
    /// Receiver of GnssReports.
    ///
    /// A value of None indicates that the modem will not send any more reports.
    reports: Option<&'c Signal<CriticalSectionRawMutex, GnssReport>>,
    power_signal: PowerSignalListener<'c>,
    _drop: AsyncDrop<'c>,

    /// The timeout value for waiting for a report.
    timeout: Duration,
}

impl<'c> Gnss<'c> {
    pub(crate) fn new(
        reports: &'c Signal<CriticalSectionRawMutex, GnssReport>,
        power_signal: PowerSignalListener<'c>,
        drop_channel: &'c DropChannel,
        timeout: Duration,
    ) -> Self {
        Gnss {
            reports: Some(reports),
            power_signal,
            _drop: AsyncDrop::new(drop_channel, DropMessage::Gnss),
            timeout,
        }
    }

    /// Wait until the next GNSS report.
    pub async fn get_report(&mut self) -> Result<GnssReport, Closed> {
        let reports = self.reports.ok_or(Closed)?;
        select_biased! {
            report = reports.wait().fuse() => Ok(report),
            _ = self.power_signal.wait_for(PowerState::Off).fuse() => {
                self.reports = None;
                Err(Closed)
            }
            _ = Timer::after(self.timeout).fuse() => {
                log::warn!("Gnss timed out");
                self.reports = None;
                Err(Closed)
            }
        }
    }

    /// Wait until the GNSS reports a fix on our location.
    pub async fn get_fix(&mut self) -> Result<GnssFix, Closed> {
        loop {
            if let GnssReport::Fix(fix) = self.get_report().await? {
                return Ok(fix);
            }
        }
    }
}
