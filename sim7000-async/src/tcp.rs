use embassy::{mutex::Mutex, blocking_mutex::raw::CriticalSectionRawMutex};
use core::future::Future;
use core::fmt::Write as WriteFmt;

use crate::{single_arc::SingletonArcGuard, write::Write, SerialError, read::Read};

pub struct TcpStream<'s, T> {
    pub instance: u8,
    pub tx: SingletonArcGuard<'s, Mutex<CriticalSectionRawMutex, T>>
}

impl<'s, T: SerialError> SerialError for TcpStream<'s, T> {
    type Error = T::Error;
}

impl<'s, T: Write> Write for TcpStream<'s, T> {
    type WriteAllFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    fn write_all<'a>(&'a mut self, words: &'a [u8]) -> Self::WriteAllFuture<'a> {
        async {
            let mut tx = self.tx.lock().await;
            let mut buf = heapless::String::<32>::new();
            write!(buf, "AT+CIPSEND={},{}\r", self.instance, words.len()).unwrap();
            tx.write_all(buf.as_bytes()).await?;

            tx.write_all(words).await?;
            tx.flush().await?;

            Ok(())
        }
    }

    fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
        async {    
            let mut tx = self.tx.lock().await;
            tx.flush().await?;
            
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
        async {
            Ok(())
        }
    }

    fn read<'a>(&'a mut self, read: &'a mut [u8]) -> Self::ReadFuture<'a> {
        async {
            Ok(0)
        }
    }
}