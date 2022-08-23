use core::sync::atomic::{AtomicBool, Ordering};

use embassy_util::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::mpmc::Channel, channel::signal::Signal,
    mutex::Mutex,
};
use heapless::Vec;

use super::{CommandRunner, RawAtCommand};
use crate::at_command::{
    response::ResponseCode,
    unsolicited::{ConnectionMessage, GnssReport, RegistrationStatus},
};
use crate::drop::DropChannel;
use crate::tcp::CONNECTION_SLOTS;

pub type TcpRxChannel = Channel<CriticalSectionRawMutex, Vec<u8, 365>, CONNECTION_SLOTS>;

pub struct ModemContext {
    pub(crate) command_lock: Mutex<CriticalSectionRawMutex, ()>,
    pub(crate) commands: Channel<CriticalSectionRawMutex, RawAtCommand, 4>,
    pub(crate) generic_response: Channel<CriticalSectionRawMutex, ResponseCode, 1>,
    pub(crate) drop_channel: DropChannel,
    pub(crate) tcp: TcpContext,
    pub(crate) registration_events: Signal<RegistrationStatus>,
    pub(crate) gnss_slot: AtomicBool,
    pub(crate) gnss_reports: Signal<GnssReport>,
}

impl ModemContext {
    pub const fn new() -> Self {
        ModemContext {
            command_lock: Mutex::new(()),
            commands: Channel::new(),
            generic_response: Channel::new(),
            drop_channel: DropChannel::new(),
            tcp: TcpContext::new(),
            registration_events: Signal::new(),
            gnss_slot: AtomicBool::new(true),
            gnss_reports: Signal::new(),
        }
    }

    pub fn commands(&self) -> CommandRunner<'_> {
        CommandRunner::create(self)
    }
}

pub struct TcpContext {
    pub(crate) rx: [TcpRxChannel; CONNECTION_SLOTS],
    pub(crate) events: [Channel<CriticalSectionRawMutex, ConnectionMessage, 4>; CONNECTION_SLOTS],
    pub(crate) slots: [AtomicBool; CONNECTION_SLOTS],
}

impl TcpContext {
    pub const fn new() -> Self {
        TcpContext {
            rx: [
                Channel::new(),
                Channel::new(),
                Channel::new(),
                Channel::new(),
                Channel::new(),
                Channel::new(),
                Channel::new(),
                Channel::new(),
            ],
            events: [
                Channel::new(),
                Channel::new(),
                Channel::new(),
                Channel::new(),
                Channel::new(),
                Channel::new(),
                Channel::new(),
                Channel::new(),
            ],
            slots: [
                AtomicBool::new(true),
                AtomicBool::new(true),
                AtomicBool::new(true),
                AtomicBool::new(true),
                AtomicBool::new(true),
                AtomicBool::new(true),
                AtomicBool::new(true),
                AtomicBool::new(true),
            ],
        }
    }
}

impl TcpContext {
    pub fn claim(&self) -> Option<TcpToken> {
        for i in 0..self.slots.len() {
            if self.slots[i].fetch_and(false, Ordering::Relaxed) {
                return Some(TcpToken {
                    ordinal: i,
                    rx: &self.rx[i],
                    events: &self.events[i],
                    slot: &self.slots[i],
                });
            }
        }

        None
    }
}

pub struct TcpToken<'c> {
    ordinal: usize,
    rx: &'c TcpRxChannel,
    events: &'c Channel<CriticalSectionRawMutex, ConnectionMessage, 4>,
    slot: &'c AtomicBool,
}

impl<'c> TcpToken<'c> {
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }

    pub fn rx(&self) -> &'c TcpRxChannel {
        self.rx
    }

    pub fn events(&self) -> &'c Channel<CriticalSectionRawMutex, ConnectionMessage, 4> {
        self.events
    }

    pub fn close(&self) {
        self.slot.fetch_or(true, Ordering::Relaxed);
    }
}
