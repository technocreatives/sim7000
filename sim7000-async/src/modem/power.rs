use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    pubsub::{DynImmediatePublisher, DynSubscriber, PubSubBehavior, PubSubChannel},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PowerState {
    On,
    Off,
    Sleeping,
}

/// A PubSub channel for signaling changes in modem power state.
///
/// Make sure that LISTENERS is high enough to accomodate your needs.
pub struct PowerSignal<const LISTENERS: usize> {
    channel: PubSubChannel<CriticalSectionRawMutex, PowerState, 2, LISTENERS, 0>,
}

pub struct PowerSignalBroadcaster<'a> {
    notifyer: DynImmediatePublisher<'a, PowerState>,
    last: PowerState,
}

pub struct PowerSignalListener<'a> {
    listener: DynSubscriber<'a, PowerState>,
}

impl<const LISTENERS: usize> PowerSignal<LISTENERS> {
    pub const fn new() -> Self {
        Self {
            channel: PubSubChannel::new(),
        }
    }

    pub fn subscribe(&self) -> PowerSignalListener<'_> {
        PowerSignalListener {
            listener: self
                .channel
                .dyn_subscriber()
                .expect("not enough PowerSignal subscribers"),
        }
    }

    pub fn publisher(&self) -> PowerSignalBroadcaster<'_> {
        PowerSignalBroadcaster {
            last: PowerState::Off,
            notifyer: self.channel.dyn_immediate_publisher(),
        }
    }

    pub fn update(&self, new_state: PowerState) {
        self.channel.publish_immediate(new_state);
    }
}

impl PowerSignalBroadcaster<'_> {
    pub async fn broadcast(&mut self, new_state: PowerState) {
        if self.last != new_state {
            self.last = new_state;
            self.notifyer.publish_immediate(new_state);
        }
    }
}

impl PowerSignalListener<'_> {
    pub async fn wait_for(&mut self, state: PowerState) {
        while self.listen().await != state {}
    }

    pub async fn wait_for_not(&mut self, state: PowerState) -> PowerState {
        loop {
            let new_state = self.listen().await;
            if new_state != state {
                return new_state;
            }
        }
    }

    pub async fn listen(&mut self) -> PowerState {
        self.listener.next_message_pure().await
    }
}
