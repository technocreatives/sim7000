use core::cmp::min;
use core::fmt::Write as WriteFmt;
use core::future::Future;
use core::mem::drop;
use embassy_util::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::mpmc::Channel, mutex::Mutex,
};
use heapless::{String, Vec};

use crate::{
    modem::{Command, TcpToken},
    read::Read,
    write::Write,
    SerialError,
};

#[derive(PartialEq, Eq)]
pub enum TcpMessage {
    SendFail,
    SendSuccess,
    Closed,
    Connected,
    ConnectionFailed,
}

#[derive(Debug)]
pub enum TcpError {
    Timeout,
    SendFail,
    Closed,
}

pub struct TcpStream<'s> {
    pub token: TcpToken<'s>,
    pub command_mutex: &'s Mutex<CriticalSectionRawMutex, ()>,
    pub commands: &'s Channel<CriticalSectionRawMutex, Command, 4>,
    pub generic_response: &'s Channel<CriticalSectionRawMutex, String<256>, 1>,
    pub closed: bool,
    pub buffer: Vec<u8, 365>,
}

impl<'s> SerialError for TcpStream<'s> {
    type Error = TcpError;
}

impl<'s> TcpStream<'s> {
    async fn send_tcp(&mut self, words: &[u8]) -> Result<(), TcpError> {
        if self.closed {
            return Err(TcpError::Closed);
        }

        let guard = self.command_mutex.lock();

        let mut buf = String::new();
        write!(buf, "AT+CIPSEND={},{}\r", self.token.ordinal(), words.len()).unwrap();
        self.commands.send(Command::Text(buf)).await;

        loop {
            match self.generic_response.recv().await.as_str() {
                "> " => break,
                "ERROR" => return Err(TcpError::SendFail),
                _ => {}
            }
        }

        let mut words = words;
        while !words.is_empty() {
            let mut chunk = Vec::new();
            let n = min(chunk.capacity(), words.len());
            chunk.extend_from_slice(&words[..n]).ok();
            words = &words[n..];
            self.commands.send(Command::Binary(chunk)).await;
        }

        loop {
            match self.token.events().recv().await {
                TcpMessage::SendFail => return Err(TcpError::SendFail),
                TcpMessage::SendSuccess => break,
                TcpMessage::Closed => {
                    self.closed = true;
                    return Err(TcpError::Closed);
                }
                _ => panic!(),
            }
        }

        drop(guard);

        Ok(())
    }

    async fn inner_read<'a>(&'a mut self, read: &'a mut [u8]) -> Result<usize, TcpError> {
        if self.closed {
            return Ok(0);
        }

        if self.buffer.is_empty() {
            let rx_buffer = loop {
                match futures_util::future::select(
                    self.token.rx().recv(),
                    self.token.events().recv(),
                )
                .await
                {
                    futures_util::future::Either::Left((buffer, _)) => break buffer,
                    futures_util::future::Either::Right((event, _))
                        if event == TcpMessage::Closed =>
                    {
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

    // TODO FIXME: if you close the stream without reading all the data, you might find that data on a
    // future unreleated stream
    pub async fn close(mut self) {
        if self.closed {
            return;
        }

        let mut buf = String::new();
        write!(buf, "AT+CIPCLOSE={}\r", self.token.ordinal()).unwrap();
        self.commands.send(Command::Text(buf)).await;

        loop {
            match self.token.events().recv().await {
                TcpMessage::Closed => break,
                _ => {}
            }
        }

        // clear read buffer
        // TODO: i'm not sure if this is enough to clear the buffer,
        // if the channel is full and the RxPump is blocked, more stuff might be added later
        while self.token.rx().try_recv().is_ok() {}

        self.closed = true;
    }
}

impl Drop for TcpStream<'_> {
    fn drop(&mut self) {
        if !self.closed {
            // I pray for async destructors
            log::warn!("TcpStream::close was not called before dropping");
        }
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

    fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
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
