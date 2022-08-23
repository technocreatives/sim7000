use core::future::Future;
use core::str::from_utf8;
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
        ModemReader {
            read,
            buffer: Vec::new(),
        }
    }

    pub async fn read_line(&mut self) -> Result<String<256>, Error> {
        const MODEM_INPUT_PROMPT: &str = "> ";
        const CRLF: &str = "\r\n";
        loop {
            if !self.buffer.is_empty() {
                log::debug!("CURRENT BUFFER {:?}", from_utf8(&self.buffer));
            }

            if self.buffer.starts_with(MODEM_INPUT_PROMPT.as_bytes()) {
                // When the modem outputs a "> " without a CRLF, it's expecting input,
                // since there is no CRLF we handle this as a special case.
                // Notably this happens after a CIPSEND command

                // Remove the prompt from the buffer
                self.buffer.rotate_left(MODEM_INPUT_PROMPT.len());
                self.buffer
                    .truncate(self.buffer.len() - MODEM_INPUT_PROMPT.len());

                return Ok(MODEM_INPUT_PROMPT.into());
            } else if let Some(position) = self
                .buffer
                .windows(CRLF.len())
                .position(|slice| slice == CRLF.as_bytes())
            {
                // If we see a line break, the modem has probaly sent us a message

                let line_end = position + CRLF.len();
                let line = from_utf8(&self.buffer[..position]).map_err(|_| Error::InvalidUtf8)?;
                log::debug!("RECV LINE: {:?}", line);

                // Ignore empty lines, as well as echoed lines ending with just a CR
                if line.trim().is_empty() || line.ends_with('\r') {
                    self.buffer.rotate_left(line_end);
                    self.buffer.truncate(self.buffer.len() - line_end);

                    continue;
                }

                let line = heapless::String::from(line);

                // Remove the line from the buffer
                self.buffer.rotate_left(line_end);
                self.buffer.truncate(self.buffer.len() - (line_end));

                return Ok(line);
            }

            if self.buffer.capacity() - self.buffer.len() == 0 {
                panic!();
            }

            let mut buf = [0u8; 256];
            let amount = self
                .read
                .read(&mut buf[..self.buffer.capacity() - self.buffer.len()])
                .await
                .map_err(|_| Error::Serial)?; // TODO: figure out error types

            self.buffer
                .extend_from_slice(&buf[..amount])
                .map_err(|_| Error::BufferOverflow)?;
        }
    }

    pub async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        if self.buffer.len() >= buf.len() {
            buf.copy_from_slice(&self.buffer.as_slice()[..buf.len()]);
            self.buffer.rotate_left(buf.len());
            self.buffer.truncate(self.buffer.len() - buf.len())
        } else {
            buf[..self.buffer.len()].copy_from_slice(self.buffer.as_slice());
            self.read
                .read_exact(&mut buf[self.buffer.len()..])
                .await
                .map_err(|_| Error::Serial)?; // TODO: figure out error types
            self.buffer.clear();
        }

        Ok(())
    }
}
