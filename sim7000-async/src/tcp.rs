use core::future::Future;
use futures_util::future::Either;
use heapless::Vec;

use crate::{
    at_command::request::cipsend,
    at_command::unsolicited::ConnectionMessage,
    drop::AsyncDrop,
    log,
    modem::{CommandRunner, TcpToken},
    read::Read,
    write::Write,
    SerialError,
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

pub struct TcpStream<'s> {
    pub(crate) token: TcpToken<'s>,
    pub(crate) _drop: AsyncDrop<'s>,
    pub(crate) commands: CommandRunner<'s>,
    pub(crate) closed: bool,
    pub(crate) buffer: Vec<u8, 365>,
}

impl<'s> SerialError for TcpStream<'s> {
    type Error = TcpError;
}

impl<'s> TcpStream<'s> {
    async fn send_tcp(&mut self, words: &[u8]) -> Result<(), TcpError> {
        if self.closed {
            return Err(TcpError::Closed);
        }

        let commands = self.commands.lock().await;

        commands
            .run(cipsend::IpSend {
                connection: self.token.ordinal(),
                data_length: words.len(),
            })
            .await
            .map_err(|_| TcpError::SendFail)?;

        commands.send_bytes(words).await;

        loop {
            match self.token.events().recv().await {
                ConnectionMessage::SendFail => return Err(TcpError::SendFail),
                ConnectionMessage::SendSuccess => break,
                ConnectionMessage::Closed => {
                    self.closed = true;
                    return Err(TcpError::Closed);
                }
                _ => panic!(),
            }
        }

        Ok(())
    }

    async fn inner_read<'a>(&'a mut self, read: &'a mut [u8]) -> Result<usize, TcpError> {
        if self.closed {
            return Ok(0);
        }

        if self.buffer.is_empty() {
            let rx_buffer = loop {
                log::info!("{} awaiting rx/event", self.token.ordinal());

                let result = futures_util::future::select(
                    self.token.rx().recv(),
                    self.token.events().recv(),
                )
                .await;

                match &result {
                    Either::Left((buffer, _)) => {
                        log::info!("{} rx got {} bytes", self.token.ordinal(), buffer.len());
                    }
                    Either::Right((event, _)) => {
                        log::info!("{} event got {:?}", self.token.ordinal(), event);
                    }
                }

                match result {
                    Either::Left((buffer, _)) => break buffer,
                    Either::Right((event, _)) if event == ConnectionMessage::Closed => {
                        self.closed = true;
                        return Ok(0);
                    }
                    _ => continue,
                }
            };

            self.buffer = rx_buffer;
        }

        if self.buffer.len() >= read.len() {
            read.copy_from_slice(&self.buffer.as_slice()[..read.len()]);
            self.buffer.rotate_left(read.len());
            self.buffer.truncate(self.buffer.len() - read.len());

            Ok(read.len())
        } else {
            read[..self.buffer.len()].copy_from_slice(self.buffer.as_slice());
            let read_len = self.buffer.len();
            self.buffer.clear();
            Ok(read_len)
        }
    }

    async fn inner_read_exact<'a>(&'a mut self, mut buf: &'a mut [u8]) -> Result<(), TcpError> {
        while !buf.is_empty() {
            match self.inner_read(buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(e) => return Err(e),
            }
        }

        if !buf.is_empty() {
            Err(TcpError::Closed)
        } else {
            Ok(())
        }
    }
}

impl Drop for TcpStream<'_> {
    fn drop(&mut self) {
        // TODO: it's likely not sufficient to clear the buffer like this,
        // if the channel is full and the RxPump is blocked, more stuff might be added later
        while self.token.rx().try_recv().is_ok() {}
    }
}

impl<'s> Write for TcpStream<'s> {
    type WriteAllFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    fn write_all<'a>(&'a mut self, words: &'a [u8]) -> Self::WriteAllFuture<'a> {
        self.send_tcp(words)
    }

    fn flush(&mut self) -> Self::FlushFuture<'_> {
        async { Ok(()) }
    }
}

impl<'s> Read for TcpStream<'s> {
    type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
    where
        Self: 'a;

    type ReadExactFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadExactFuture<'a> {
        self.inner_read_exact(buf)
    }

    fn read<'a>(&'a mut self, read: &'a mut [u8]) -> Self::ReadFuture<'a> {
        self.inner_read(read)
    }
}
