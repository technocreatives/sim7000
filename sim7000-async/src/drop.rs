use core::mem::drop;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

use crate::at_command::request::{CloseConnection, SetGnssPower};
use crate::gnss::GNSS_SLOTS;
use crate::modem::{CommandRunnerGuard, ModemContext};
use crate::tcp::CONNECTION_SLOTS;
use crate::Error;

/// The capacity of the drop channel
/// Nust be at least the number of unique objects that can be dropped.
const DROP_CAPACITY: usize = GNSS_SLOTS + CONNECTION_SLOTS;
pub type DropChannel = Channel<CriticalSectionRawMutex, DropMessage, DROP_CAPACITY>;

pub struct AsyncDrop<'c> {
    channel: &'c DropChannel,
    message: DropMessage,
}

impl<'c> AsyncDrop<'c> {
    pub fn new(channel: &'c DropChannel, message: DropMessage) -> Self {
        AsyncDrop { channel, message }
    }
}

impl Drop for AsyncDrop<'_> {
    fn drop(&mut self) {
        if self.channel.try_send(self.message).is_err() {
            log::error!("Failed to drop {:?}: Drop channel full", self.message);
        }
    }
}

/// This type facilitates asynchronous dropping.
#[derive(Clone, Copy, Debug)]
pub enum DropMessage {
    Connection(usize),
    Gnss,
}

impl DropMessage {
    pub async fn run(&self, runner: &mut CommandRunnerGuard<'_>) -> Result<(), Error> {
        log::debug!("Sending drop command for {self:?}");

        /// It is Ok for the result to be a SimError
        fn sim_may_fail(error: Error) -> Result<(), Error> {
            match error {
                Error::Sim(_) => Ok(()),
                _ => Err(error),
            }
        }

        match self {
            &DropMessage::Connection(n) => {
                runner
                    .run(CloseConnection { connection: n })
                    .await
                    .map(drop)
                    // Closing is allowed to fail, since this might happen if the connection was
                    // closed on the remote before we closed it here.
                    .or_else(sim_may_fail)
            }
            DropMessage::Gnss => runner.run(SetGnssPower(false)).await.map(drop),
        }
    }

    pub fn clean_up(&self, ctx: &ModemContext) {
        log::debug!("Cleaning up after {self:?}");
        match self {
            &DropMessage::Connection(n) => {
                ctx.tcp.slots[n].release();
            }
            DropMessage::Gnss => {
                ctx.gnss_slot.release();
            }
        }
    }
}
