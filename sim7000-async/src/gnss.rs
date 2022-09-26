use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

use crate::at_command::unsolicited::{GnssFix, GnssReport};
use crate::drop::{AsyncDrop, DropChannel, DropMessage};

pub const GNSS_SLOTS: usize = 1;

pub struct Gnss<'c> {
    pub(crate) _drop: AsyncDrop<'c>,
    pub(crate) reports: &'c Signal<CriticalSectionRawMutex, GnssReport>,
}

impl<'c> Gnss<'c> {
    pub fn new(
        reports: &'c Signal<CriticalSectionRawMutex, GnssReport>,
        drop_channel: &'c DropChannel,
    ) -> Self {
        Gnss {
            _drop: AsyncDrop::new(drop_channel, DropMessage::Gnss),
            reports,
        }
    }

    /// Wait until the next GNSS report.
    pub async fn get_report(&self) -> GnssReport {
        self.reports.wait().await
    }

    /// Wait until the GNSS reports a fix on our location.
    pub async fn get_fix(&self) -> GnssFix {
        loop {
            if let GnssReport::Fix(fix) = self.reports.wait().await {
                return fix;
            }
        }
    }
}
