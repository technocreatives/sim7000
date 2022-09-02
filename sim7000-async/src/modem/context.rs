use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, mutex::Mutex, signal::Signal, pipe::Pipe,
};
use heapless::Vec;

use super::{CommandRunner, RawAtCommand};
use crate::at_command::{
    response::ResponseCode,
    unsolicited::{ConnectionMessage, GnssReport, RegistrationStatus, VoltageWarning},
};
use crate::drop::DropChannel;
use crate::slot::Slot;
use crate::tcp::CONNECTION_SLOTS;

pub type TcpRxChannel = Channel<CriticalSectionRawMutex, Vec<u8, 365>, 8>;
pub type TcpEventChannel = Channel<CriticalSectionRawMutex, ConnectionMessage, 8>;

pub struct ModemContext {
    pub(crate) command_lock: Mutex<CriticalSectionRawMutex, ()>,
    pub(crate) commands: Channel<CriticalSectionRawMutex, RawAtCommand, 4>,
    pub(crate) generic_response: Channel<CriticalSectionRawMutex, ResponseCode, 1>,
    pub(crate) drop_channel: DropChannel,
    pub(crate) tcp: TcpContext,
    pub(crate) registration_events: Signal<RegistrationStatus>,
    pub(crate) gnss_slot: Slot<Signal<GnssReport>>,
    pub(crate) voltage_slot: Slot<Signal<VoltageWarning>>,
    pub(crate) tx_pipe: Pipe<CriticalSectionRawMutex, 2048>,
    pub(crate) rx_pipe: Pipe<CriticalSectionRawMutex, 2048>,
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
            gnss_slot: Slot::new(Signal::new()),
            voltage_slot: Slot::new(Signal::new()),
            tx_pipe: Pipe::new(),
            rx_pipe: Pipe::new(),
        }
    }

    pub fn commands(&self) -> CommandRunner<'_> {
        CommandRunner::create(self)
    }
}

pub struct TcpSlot {
    pub rx: TcpRxChannel,
    pub events: TcpEventChannel,
}

pub struct TcpContext {
    pub(crate) slots: [Slot<TcpSlot>; CONNECTION_SLOTS],
}

impl TcpSlot {
    pub const fn new() -> Self {
        TcpSlot {
            rx: Channel::new(),
            events: Channel::new(),
        }
    }
}

impl TcpContext {
    pub const fn new() -> Self {
        TcpContext {
            slots: [
                Slot::new(TcpSlot::new()),
                Slot::new(TcpSlot::new()),
                Slot::new(TcpSlot::new()),
                Slot::new(TcpSlot::new()),
                Slot::new(TcpSlot::new()),
                Slot::new(TcpSlot::new()),
                Slot::new(TcpSlot::new()),
                Slot::new(TcpSlot::new()),
            ],
        }
    }
}

impl TcpContext {
    pub fn claim(&self) -> Option<TcpToken> {
        self.slots.iter().enumerate().find_map(|(i, slot)| {
            let TcpSlot { rx, events } = slot.claim()?; // find an unclaimed slot
            Some(TcpToken {
                ordinal: i,
                rx,
                events,
            })
        })
    }
}

pub struct TcpToken<'c> {
    ordinal: usize,
    rx: &'c TcpRxChannel,
    events: &'c TcpEventChannel,
}

impl<'c> TcpToken<'c> {
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }

    pub fn rx(&self) -> &'c TcpRxChannel {
        self.rx
    }

    pub fn events(&self) -> &'c TcpEventChannel {
        self.events
    }
}
