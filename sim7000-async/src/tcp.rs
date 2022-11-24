use core::{
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub::PubSubChannel};
use embedded_io::{
    asynch::{Read, Write},
    Io,
};
use futures_util::FutureExt;

use crate::{
    at_command::{cipsend, unsolicited::ConnectionMessage},
    drop::{AsyncDrop, DropChannel, DropMessage},
    log,
    modem::{CommandRunner, TcpToken},
};

/// The maximum number of parallel connections supported by the modem
pub const CONNECTION_SLOTS: usize = 8;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TcpError {
    Timeout,
    SendFail,
    Closed,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ConnectError {
    ConnectFailed,
    Other(crate::Error),
}

impl embedded_io::Error for TcpError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

impl From<crate::Error> for ConnectError {
    fn from(e: crate::Error) -> Self {
        ConnectError::Other(e)
    }
}

pub struct TcpStream<'s> {
    token: TcpToken<'s>,
    _drop: AsyncDrop<'s>,
    commands: CommandRunner<'s>,
    closed: AtomicBool,
    events: PubSubChannel<CriticalSectionRawMutex, ConnectionMessage, 1, 2, 2>,
}

pub struct TcpReader<'s> {
    stream: &'s TcpStream<'s>,
}

pub struct TcpWriter<'s> {
    stream: &'s TcpStream<'s>,
}

impl<'s> Io for TcpStream<'s> {
    type Error = TcpError;
}
impl<'s> Io for TcpWriter<'s> {
    type Error = TcpError;
}
impl<'s> Io for TcpReader<'s> {
    type Error = TcpError;
}

impl Drop for TcpStream<'_> {
    fn drop(&mut self) {
        // TODO: it's likely not sufficient to clear the buffer like this,
        // if the channel is full and the RxPump is blocked, more stuff might be added later
        self.token.rx().clear();
    }
}

impl<'s> TcpStream<'s> {
    pub(crate) fn new(
        token: TcpToken<'s>,
        drop_channel: &'s DropChannel,
        commands: CommandRunner<'s>,
    ) -> Self {
        TcpStream {
            _drop: AsyncDrop::new(drop_channel, DropMessage::Connection(token.ordinal())),
            token,
            commands,
            closed: AtomicBool::new(false),
            events: PubSubChannel::new(),
        }
    }

    pub fn split(&mut self) -> (TcpReader<'_>, TcpWriter<'_>) {
        let reader = TcpReader { stream: self };
        let writer = TcpWriter { stream: self };
        (reader, writer)
    }

    /// Listen to, and forward tcp events to both the read and the write half of this stream.
    ///
    /// Must be used in a select in combination with awaiting `TcpStream.events`
    async fn handle_events(&self) -> ! {
        let publisher =
            self.events.publisher().unwrap(/* capacity is 2, only reader and writer use channel */);
        loop {
            let event = self.token.events().recv().await;
            publisher.publish(event).await;
        }
    }
}

impl<'s> Write for TcpStream<'s> {
    type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
    where
        Self: 'a;

    type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
        async {
            let (_, mut writer) = self.split();
            writer.write(buf).await
        }
    }

    fn flush(&mut self) -> Self::FlushFuture<'_> {
        async { Ok(()) }
    }
}

impl<'s> Read for TcpStream<'s> {
    type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
    where
        Self: 'a;

    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
        async {
            let (mut reader, _) = self.split();
            reader.read(buf).await
        }
    }
}

impl<'s> Write for TcpWriter<'s> {
    type WriteFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
    where
        Self: 'a;

    type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    fn write<'a>(&'a mut self, buf: &'a [u8]) -> Self::WriteFuture<'a> {
        async {
            let stream = self.stream;
            let mut events = stream
                .events
                .subscriber()
                .expect("claim tcp stream event subscriber");

            /// The maximum number of bytes the modem can handle in a single CIPSEND command
            // TODO: I *think* this is configurable in the modem, if so, we should check what this
            // value actually is.
            const MODEM_WRITE_BUF: usize = 1024;

            for chunk in buf.chunks(MODEM_WRITE_BUF) {
                if stream.closed.load(Ordering::Acquire) {
                    return Err(TcpError::Closed);
                }

                let commands = stream.commands.lock().await;

                commands
                    .run(cipsend::IpSend {
                        connection: stream.token.ordinal(),
                        data_length: chunk.len(),
                    })
                    .await
                    .map_err(|_| TcpError::SendFail)?;

                commands.send_bytes(chunk).await;

                loop {
                    futures::select_biased! {
                        _ = stream.handle_events().fuse() => unreachable!(),
                        event = events.next_message_pure().fuse() => match event {
                            ConnectionMessage::SendFail => return Err(TcpError::SendFail),
                            ConnectionMessage::SendSuccess => break,
                            ConnectionMessage::Closed => {
                                stream.closed.store(true, Ordering::Release);
                                return Err(TcpError::Closed);
                            }
                            _ => {}
                        }
                    }
                }
            }

            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> Self::FlushFuture<'_> {
        async { Ok(()) }
    }
}

impl<'s> Read for TcpReader<'s> {
    type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
    where
        Self: 'a;

    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadFuture<'a> {
        async {
            let stream = self.stream;

            let mut events = stream
                .events
                .subscriber()
                .expect("claim tcp stream event subscriber");

            if stream.closed.load(Ordering::Acquire) {
                return Ok(0);
            }

            loop {
                log::trace!("tcp {} awaiting rx/event", stream.token.ordinal());

                futures::select_biased! {
                    _ = stream.handle_events().fuse() => unreachable!(),
                    n = stream.token.rx().read(buf).fuse() => {
                        log::trace!("tcp {} rx got {} bytes", stream.token.ordinal(), n);
                        break Ok(n);
                    }
                    event = events.next_message_pure().fuse() => {
                        log::trace!("tcp {} got event {:?}", stream.token.ordinal(), event);
                        if event == ConnectionMessage::Closed {
                            stream.closed.store(true, Ordering::Release);
                            break Ok(0);
                        }
                    }
                };
            }
        }
    }
}
