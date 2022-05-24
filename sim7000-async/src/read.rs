use core::{future::Future};

use heapless::{String, Vec};

use crate::{Error, SerialError};

pub trait Read: SerialError {
    /// Future returned by the `read` method.
    type ReadFuture<'a>: Future<Output = Result<usize, Self::Error>> + 'a
    where
        Self: 'a;

    type ReadExactFuture<'a>: Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadExactFuture<'a>;

    /// Reads words from the serial interface into the supplied slice.
    fn read<'a>(&'a mut self, read: &'a mut [u8]) -> Self::ReadFuture<'a>;
}

pub struct ModemReader<R> {
    read: R,
    buffer: Vec<u8, 256>,
}

impl<R: Read> ModemReader<R> {
    pub fn new(read: R) -> ModemReader<R> {
        ModemReader { read, buffer: Vec::new() }
    }
    
    pub async fn read_line(&mut self) -> Result<String<256>, Error<R::Error>> {
        loop {
            log::debug!("CURRENT BUFFER {:?}", core::str::from_utf8(&self.buffer));
            if let Some(position) = self.buffer.windows(2).position(|slice| slice == b"\r\n") {
                self.buffer.rotate_left(position);
                self.buffer.truncate(self.buffer.len() - position);

                if let Some(position) = self.buffer[2..]
                    .windows(2)
                    .position(|slice| slice == b"\r\n")
                {
                    let line_end = position + 2;
                    let s = core::str::from_utf8(&self.buffer[2..line_end])
                        .map_err(|_| Error::InvalidUtf8)?;
                    log::debug!("RECV LINE: {:?}", s);
                    
                    // the sim7000 doesn't remember hardware flow control settings so during initialization it might drop bytes. This will fix a misaligned line reader since the sim7000 never sends empty messages
                    if s.is_empty() {
                        self.buffer.rotate_left(line_end);
                        self.buffer.truncate(self.buffer.len() - line_end);

                        continue;
                    }
                    let line = heapless::String::from(s);

                    self.buffer.rotate_left(line_end + 2);
                    self.buffer.truncate(self.buffer.len() - (line_end + 2));

                    return Ok(line);
                }
            }

            if self.buffer.capacity() - self.buffer.len() == 0 {
                panic!();
            }

            let mut buf = [0u8; 256];
            let amount = self
                .read
                .read(&mut buf[..self.buffer.capacity() - self.buffer.len()])
                .await?;

            self.buffer
                .extend_from_slice(&buf[..amount])
                .map_err(|_| Error::BufferOverflow)?;
        }
    }

    pub async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error<R::Error>> {
        if self.buffer.len() >= buf.len() {
            buf.copy_from_slice(&self.buffer.as_slice()[..buf.len()]);
            self.buffer.rotate_left(buf.len());
            self.buffer.truncate(self.buffer.len() - buf.len())
        } else {
            buf[..self.buffer.len()].copy_from_slice(self.buffer.as_slice());
            self.read.read_exact(&mut buf[self.buffer.len()..]).await?;
            self.buffer.clear();
        }

        Ok(())
    }
}
