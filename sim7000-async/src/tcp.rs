use embassy::{mutex::Mutex, blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, time::Duration};
use heapless::{Vec, String};
use core::future::Future;
use core::fmt::Write as WriteFmt;

use crate::{single_arc::SingletonArcGuard, write::Write, SerialError, read::Read, modem::{TcpRxChannel, TcpToken}};

#[derive(PartialEq, Eq)]
pub enum TcpMessage {
    SendFail,
    SendSuccess,
    Closed,
    Connected,
    ConnectionFailed,
}

#[derive(Debug)]
pub enum TcpError<T> {
    Timeout,
    SendFail,
    Closed,
    Io(T),
}

impl<T> From<T> for TcpError<T> {
    fn from(inner: T) -> Self {
        TcpError::Io(inner)
    }
}

pub struct TcpStream<'s, T> {
    pub token: TcpToken<'s>,
    pub tx: SingletonArcGuard<'s, Mutex<CriticalSectionRawMutex, T>>,
    pub closed: bool,
    pub buffer: Vec<u8, 365>,
}

impl<'s, T: SerialError> SerialError for TcpStream<'s, T> {
    type Error = TcpError<T::Error>;
}

impl<'s, T: Write> TcpStream<'s, T> {
    async fn send_tcp(&mut self, words: &[u8]) -> Result<(), TcpError<T::Error>> {
        if self.closed {
            return Err(TcpError::Closed);
        }
        
        let mut tx = self.tx.lock().await;
        let mut buf = heapless::String::<32>::new();
        write!(buf, "AT+CIPSEND={},{}\r", self.token.ordinal(), words.len()).unwrap();
        log::debug!("WRITING DATA");
        tx.write_all(buf.as_bytes()).await?;
        embassy::time::Timer::after(Duration::from_millis(2000)).await;
        tx.write_all(words).await?;

        tx.flush().await?;

        write!(buf, "AT+CIPSEND={},{}\r", self.token.ordinal(), words.len()).unwrap();
        log::debug!("WRITING DATA");
        //tx.write_all(buf.as_bytes()).await?;
        //tx.write_all(words).await?;

        //tx.flush().await?;
        log::debug!("WAITING FOR SEND OK");

        loop {
            match self.token.events().recv().await {
                TcpMessage::SendFail => return Err(TcpError::SendFail),
                TcpMessage::SendSuccess => break,
                TcpMessage::Closed => {
                    self.closed = true;
                    return Err(TcpError::Closed);
                },
                _ => panic!(),
            }
        }
        drop(tx);

        Ok(())
    }

    async fn inner_read<'a>(&'a mut self, read: &'a mut [u8]) -> Result<usize, TcpError<T::Error>> {
            if self.closed {
                return Ok(0);
            }

            if self.buffer.is_empty() {
                let rx_buffer = loop {
                    match futures_util::future::select(self.token.rx().recv(), self.token.events().recv()).await {
                        futures_util::future::Either::Left((buffer, _)) => break buffer,
                        futures_util::future::Either::Right((event, _)) if event == TcpMessage::Closed => {
                            self.closed = true;
                            return Ok(0);
                        },
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

    async fn inner_read_exact<'a>(&'a mut self, mut buf: &'a mut [u8]) -> Result<(), TcpError<T::Error>> {
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

    pub async fn close(self) {
        let mut tx = self.tx.lock().await;
        let mut buf = heapless::String::<32>::new();
        write!(buf, "AT+CIPCLOSE={}\r", self.token.ordinal()).unwrap();

        tx.write_all(buf.as_bytes()).await.unwrap();

        loop {match self.token.events().recv().await {
            TcpMessage::Closed => break,
            _ => {}
        }}
    }
}

impl<'s, T: Write> Write for TcpStream<'s, T> {
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
        async {    
            Ok(())
        }
    }
}

impl<'s, T: Write> Read for TcpStream<'s, T> {
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