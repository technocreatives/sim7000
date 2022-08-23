use embassy_util::channel::signal::Signal;

use crate::at_command::unsolicited::GnssReport;
use crate::drop::{AsyncDrop, DropChannel, DropMessage};

pub const GNSS_SLOTS: usize = 1;

pub struct Gnss<'c> {
    pub(crate) _drop: AsyncDrop<'c>,
    pub(crate) reports: &'c Signal<GnssReport>,
}

impl<'c> Gnss<'c> {
    pub fn new(reports: &'c Signal<GnssReport>, drop_channel: &'c DropChannel) -> Self {
        Gnss {
            _drop: AsyncDrop::new(drop_channel, DropMessage::Gnss),
            reports,
        }
    }
    pub async fn report(&self) -> GnssReport {
        self.reports.wait().await
    }
}
