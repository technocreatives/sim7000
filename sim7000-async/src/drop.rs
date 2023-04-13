//! Types for facilitating asynchronous dropping.
//!
//! For certain types it is necessary to run the descructor asynchronously. This is impossible with
//! the regular Drop trait.
//!
//! This module provides the [AsyncDrop] type, which takes a [DropChannel] and a [DropMessage], and
//! all it does is pass the [DropMessage] to the channel when the [AsyncDrop] is dropped. The
//! intended use-case is embedding the [AsyncDrop] in another struct, and then letting the drop
//! logic happen implicitly.
//!
//! The actual drop-logic will take place on another task dedicated to polling the [DropChannel]
//! (see [DropPump](crate::pump::DropPump)).
//!
//! Example
//! ```ignore
//! struct TypeWithAnAsyncDescructor {
//!    _drop: AsyncDrop<'static>,
//! }
//! ```

use core::mem::drop;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

use crate::at_command::{CloseConnection, SetGnssPower};
use crate::gnss::GNSS_SLOTS;
use crate::log;
use crate::modem::{CommandRunnerGuard, ModemContext};
use crate::tcp::CONNECTION_SLOTS;
use crate::Error;

/// The capacity of the drop channel.
/// Nust be at least the number of unique objects that can be dropped.
const DROP_CAPACITY: usize = GNSS_SLOTS + CONNECTION_SLOTS;
pub type DropChannel = Channel<CriticalSectionRawMutex, DropMessage, DROP_CAPACITY>;

/// Type for facilitating asynchronous dropping. See module-level docs for details.
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

/// This type facilitates asynchronous dropping. See module-level docs for details.
///
/// This enum has one variant for each kind of type which needs asynchronous dropping.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DropMessage {
    /// Drop a [TcpStream](crate::tcp::TcpStream).
    Connection(usize),

    /// Drop a [Gnss](crate::gnss::Gnss).
    Gnss,
}

impl DropMessage {
    /// Run the Drop logic for this message.
    pub async fn run(&self, runner: &mut CommandRunnerGuard<'_>) -> Result<(), Error> {
        log::debug!("Sending drop command for {:?}", self);

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
                    // Closing is allowed to fail, since this might happen if the connection was
                    // closed on the remote before we closed it here.
                    .map(drop)
                    .or_else(sim_may_fail)?;
            }
            DropMessage::Gnss => {
                runner.run(SetGnssPower(false)).await?;
            }
        }

        Ok(())
    }

    /// Clean up logic that gets run after running the drop logic, regardless of whether the drop
    /// logic errored or not.
    pub fn clean_up(&self, ctx: &ModemContext) {
        log::debug!("Cleaning up after {:?}", self);
        match self {
            &DropMessage::Connection(n) => {
                let tcp_ctx = ctx.tcp.slots[n].peek();
                tcp_ctx.rx.clear();
                tcp_ctx.events.clear();
                ctx.tcp.slots[n].release();
            }
            DropMessage::Gnss => {
                ctx.gnss_slot.release();
            }
        }
    }
}
