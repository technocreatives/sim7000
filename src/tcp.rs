use cipstart::ConnectMode;
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub::PubSubChannel};
use embassy_time::{with_timeout, Duration, TimeoutError, Timer};
use embedded_io_async::{
    ErrorType, {Read, Write},
};
use futures::{select_biased, FutureExt};

use crate::{
    at_command::{at, cipsend, cipstart, unsolicited::ConnectionMessage, At},
    drop::{AsyncDrop, DropChannel, DropMessage},
    log,
    modem::{CommandRunner, TcpToken},
    util::Lagged,
    Error,
};

/// The maximum number of concurrent TCP connections supported by the modem.
pub const MAX_TCP_SLOTS: usize = 8;

/// The number of bytes allocated for each TCP slot receive buffer.
pub const TCP_RX_BUF_LEN: usize = 3072;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TcpError {
    Timeout,
    SendFail,
    Closed,
}

impl embedded_io_async::Error for TcpError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        match self {
            TcpError::Timeout => embedded_io_async::ErrorKind::TimedOut,
            TcpError::SendFail => embedded_io_async::ErrorKind::Other,
            TcpError::Closed => embedded_io_async::ErrorKind::Other,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[non_exhaustive]
pub enum ConnectError {
    ConnectFailed,

    /// No connection slots available, the max number of open connections has been reached.
    /// For TCP, this number is [MAX_TCP_SLOTS], and is a hard limit set by the modem.
    NoFreeSlots,

    Other(crate::Error),

    /// The modem gave an unexpected response
    Unexpected(ConnectionMessage),
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

    /// Whether the stream is closed
    closed: AtomicBool,

    /// A channel to proxy ConnectionMessages to both the TcpWriter and TcpReader.
    events: PubSubChannel<CriticalSectionRawMutex, ConnectionMessage, 1, 2, 2>,

    /// Timeout of read/write operations
    timeout: Duration,
}

pub struct TcpReader<'s> {
    stream: &'s TcpStream<'s>,
}

pub struct TcpWriter<'s> {
    stream: &'s TcpStream<'s>,
}

impl ErrorType for TcpStream<'_> {
    type Error = TcpError;
}
impl ErrorType for TcpWriter<'_> {
    type Error = TcpError;
}
impl ErrorType for TcpReader<'_> {
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
    pub(crate) async fn connect(
        token: TcpToken<'s>,
        host: &str,
        port: u16,
        drop_channel: &'s DropChannel,
        commands: CommandRunner<'s>,
    ) -> Result<TcpStream<'s>, ConnectError> {
        // create a drop guard here, so that if this function errors,
        // we make sure to clean up the connection
        let drop_guard = AsyncDrop::new(drop_channel, DropMessage::Connection(token.ordinal()));

        commands
            .lock()
            .await
            .run(cipstart::Connect {
                mode: ConnectMode::Tcp,
                number: token.ordinal(),
                port,

                #[allow(clippy::unnecessary_fallible_conversions)] // heapless string panics on from
                destination: host.try_into().map_err(|_| Error::BufferOverflow)?,
            })
            .await?;

        // Wait for a response.
        // Based on testing, a connection will timeout after ~120 seconds, so we add our own
        // timeout to this step to prevent us from waiting forever if the modem died.
        for _ in 0..21 {
            match with_timeout(Duration::from_secs(6), token.next_message()).await {
                Err(TimeoutError) => {
                    // Make sure the modem is still responding to commands.
                    commands.lock().await.run(at::At).await?;
                }
                Ok(Err(Lagged)) => {
                    log::error!(
                        "TcpStream lagged while waiting to establish a connection. \
                         This shouldn't happen. Is the executor extremely overloaded?"
                    );
                }
                Ok(Ok(msg)) => match msg {
                    ConnectionMessage::Connected => {
                        return Ok(TcpStream {
                            _drop: drop_guard,
                            token,
                            commands,
                            closed: AtomicBool::new(false),
                            events: PubSubChannel::new(),
                            timeout: Duration::from_secs(120),
                        });
                    }

                    ConnectionMessage::ConnectionFailed => return Err(ConnectError::ConnectFailed),

                    // This should never happen, since we guard against connections already being used.
                    msg => return Err(ConnectError::Unexpected(msg)),
                },
            }
        }

        // The modem never got back to us, it probably died.
        Err(ConnectError::Other(Error::Timeout))
    }

    /// Set the timeout of read and write operations.
    ///
    /// Default is 120 seconds.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Split the stream into a reader and a writer half.
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
            match self.token.next_message().await {
                Ok(message) => publisher.publish(message).await,
                Err(Lagged) => {
                    log::warn!(
                        "TcpStream {} missed some connection messages from the modem. \
                        This shouldn't happen, this connection may behave unpredictably.",
                        self.token.ordinal()
                    );
                }
            }
        }
    }
}

impl Write for TcpStream<'_> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let (_, mut writer) = self.split();
        writer.write(buf).await
    }
}

impl Read for TcpStream<'_> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let (mut reader, _) = self.split();
        reader.read(buf).await
    }
}

impl Write for TcpWriter<'_> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
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

            use ConnectionMessage::*;
            select_biased! {
                _ = stream.handle_events().fuse() => unreachable!(),
                event = events.next_message_pure().fuse() => match event {
                    SendSuccess => {},
                    SendFail => return Err(TcpError::SendFail),
                    Closed => {
                        stream.closed.store(true, Ordering::Release);
                        return Err(TcpError::Closed);
                    }
                    Connected | AlreadyConnected | ConnectionFailed => {
                        log::error!("TcpStream received an unexpected ConnectionMessage: {:?}", event);
                        stream.closed.store(true, Ordering::Release);
                        return Err(TcpError::Closed);
                    }
                },
                _ = Timer::after(self.stream.timeout).fuse() => {
                    return Err(TcpError::Timeout);
                }
            }
        }

        Ok(buf.len())
    }
}

impl Read for TcpReader<'_> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
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

            select_biased! {
                _ = stream.handle_events().fuse() => unreachable!(),
                n = stream.token.rx().read(buf).fuse() => {
                    log::trace!("tcp {} rx got {} bytes", stream.token.ordinal(), n);
                    break Ok(n);
                }
                event = events.next_message_pure().fuse() => {
                    log::trace!("tcp {} got event {:?}", stream.token.ordinal(), event);
                    use ConnectionMessage::*;
                    match event {
                        Closed => {
                            stream.closed.store(true, Ordering::Release);
                            break Ok(0);
                        }
                        SendSuccess | SendFail => {}
                        Connected | AlreadyConnected | ConnectionFailed => {
                            log::error!("TcpStream received an unexpected ConnectionMessage: {:?}", event);
                            stream.closed.store(true, Ordering::Release);
                            return Err(TcpError::Closed);
                        }
                    }
                }
                _ = Timer::after(self.stream.timeout).fuse() => {
                    let commands = self.stream.commands.lock().await;

                    // make sure the modem is still alive
                    if commands.run(At).await.is_err() {
                        return Err(TcpError::Timeout);
                    }
                }
            };
        }
    }
}
