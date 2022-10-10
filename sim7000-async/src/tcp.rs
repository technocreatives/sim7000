use core::{
    future::Future,
    sync::atomic::{AtomicBool, Ordering},
};
use embedded_io::{
    asynch::{Read, Write},
    Io,
};
use futures_util::future::{select, Either};

use crate::{
    at_command::{cipsend, unsolicited::ConnectionMessage},
    drop::AsyncDrop,
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
    pub(crate) token: TcpToken<'s>,
    pub(crate) _drop: AsyncDrop<'s>,
    pub(crate) commands: CommandRunner<'s>,
    pub(crate) closed: AtomicBool,
}

pub struct TcpReader<'s> {
    token: &'s TcpToken<'s>,
    closed: &'s AtomicBool,
}

pub struct TcpWriter<'s> {
    token: &'s TcpToken<'s>,
    closed: &'s AtomicBool,
    commands: &'s CommandRunner<'s>,
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
    pub fn split(&mut self) -> (TcpReader<'_>, TcpWriter<'_>) {
        let reader = TcpReader {
            token: &self.token,
            closed: &self.closed,
        };

        let writer = TcpWriter {
            token: &self.token,
            closed: &self.closed,
            commands: &self.commands,
        };

        (reader, writer)
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
            if self.closed.load(Ordering::Acquire) {
                return Err(TcpError::Closed);
            }

            let commands = self.commands.lock().await;

            commands
                .run(cipsend::IpSend {
                    connection: self.token.ordinal(),
                    data_length: buf.len(),
                })
                .await
                .map_err(|_| TcpError::SendFail)?;

            commands.send_bytes(buf).await;

            loop {
                match self.token.events().recv().await {
                    ConnectionMessage::SendFail => return Err(TcpError::SendFail),
                    ConnectionMessage::SendSuccess => break,
                    ConnectionMessage::Closed => {
                        self.closed.store(true, Ordering::Release);
                        return Err(TcpError::Closed);
                    }
                    _ => panic!(),
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
            if self.closed.load(Ordering::Acquire) {
                return Ok(0);
            }

            let result = select(self.token.rx().read(buf), self.token.events().recv()).await;

            log::info!("{} awaiting rx/event", self.token.ordinal());

            loop {
                match &result {
                    Either::Left((n, _)) => {
                        log::info!("{} rx got {} bytes", self.token.ordinal(), n);
                    }
                    Either::Right((event, _)) => {
                        log::info!("{} event got {:?}", self.token.ordinal(), event);
                    }
                }

                match result {
                    Either::Left((n, _)) => break Ok(n),
                    Either::Right((event, _)) if event == ConnectionMessage::Closed => {
                        self.closed.store(true, Ordering::Release);
                        break Ok(0);
                    }
                    _ => continue,
                }
            }
        }
    }
}
