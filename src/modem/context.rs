use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, mutex::Mutex, pipe::Pipe,
    signal::Signal,
};

use super::{power::PowerSignal, CommandRunner, RawAtCommand};
use crate::{
    at_command::unsolicited::RegistrationStatus,
    at_command::{
        unsolicited::{ConnectionMessage, GnssReport, NetworkRegistration, VoltageWarning},
        ResponseCode,
    },
    drop::DropChannel,
    slot::Slot,
    tcp::TCP_RX_BUF_LEN,
    util::Lagged,
    util::RingChannel,
    StateSignal,
};

pub type TcpRxPipe = Pipe<CriticalSectionRawMutex, TCP_RX_BUF_LEN>;
pub type TcpEventChannel = RingChannel<CriticalSectionRawMutex, ConnectionMessage, 8>;

pub struct ModemContext {
    pub(crate) power_signal: PowerSignal,
    pub(crate) command_lock: Mutex<CriticalSectionRawMutex, ()>,
    pub(crate) commands: Channel<CriticalSectionRawMutex, RawAtCommand, 4>,
    pub(crate) generic_response: Channel<CriticalSectionRawMutex, ResponseCode, 1>,
    pub(crate) drop_channel: DropChannel,
    pub(crate) tcp: TcpContext,
    pub(crate) registration_events: StateSignal<CriticalSectionRawMutex, NetworkRegistration>,
    pub(crate) gnss_slot: Slot<Signal<CriticalSectionRawMutex, GnssReport>>,
    pub(crate) voltage_slot: Slot<Signal<CriticalSectionRawMutex, VoltageWarning>>,
    pub(crate) tx_pipe: Pipe<CriticalSectionRawMutex, 2048>,
    pub(crate) rx_pipe: Pipe<CriticalSectionRawMutex, 2048>,
}

impl ModemContext {
    pub const fn new(tcp: TcpContext) -> Self {
        ModemContext {
            power_signal: PowerSignal::new(),
            command_lock: Mutex::new(()),
            commands: Channel::new(),
            generic_response: Channel::new(),
            drop_channel: DropChannel::new(),
            tcp,
            registration_events: StateSignal::new(NetworkRegistration {
                status: RegistrationStatus::Unknown,
                lac: None,
                ci: None,
            }),
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
    pub rx: TcpRxPipe,
    pub events: TcpEventChannel,
}

pub struct TcpContext {
    pub(crate) slots: &'static [Slot<TcpSlot>],
}

impl TcpSlot {
    pub const fn new() -> Self {
        TcpSlot {
            rx: Pipe::new(),
            events: TcpEventChannel::new(),
        }
    }
}

impl TcpContext {
    pub const fn new(slots: &'static [Slot<TcpSlot>]) -> Self {
        TcpContext { slots }
    }

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

    pub async fn disconnect_all(&self) {
        for slot in self.slots {
            if slot.is_claimed() {
                slot.peek().events.send(ConnectionMessage::Closed);
            }
        }
    }
}

pub struct TcpToken<'c> {
    ordinal: usize,
    rx: &'c TcpRxPipe,
    events: &'c RingChannel<CriticalSectionRawMutex, ConnectionMessage, 8>,
}

impl<'c> TcpToken<'c> {
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }

    pub fn rx(&self) -> &'c TcpRxPipe {
        self.rx
    }

    pub async fn next_message(&self) -> Result<ConnectionMessage, Lagged> {
        self.events.recv().await
    }
}
