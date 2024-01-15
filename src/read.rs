use core::str::from_utf8;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pipe::Pipe};
use embedded_io_async::Read;
use heapless::{String, Vec};

use crate::{log, Error};

pub struct ModemReader<'context> {
    read: &'context Pipe<CriticalSectionRawMutex, 2048>,
    buffer: Vec<u8, 256>,
}

impl<'context> ModemReader<'context> {
    pub fn new(read: &'context Pipe<CriticalSectionRawMutex, 2048>) -> ModemReader<'context> {
        ModemReader {
            read,
            buffer: Vec::new(),
        }
    }

    pub async fn read_line(&mut self) -> Result<String<256>, Error> {
        const MODEM_INPUT_PROMPT: &str = "> ";
        const LINE_END: &str = "\n";
        loop {
            if !self.buffer.is_empty() {
                match from_utf8(&self.buffer) {
                    Ok(line) => log::trace!("CURRENT BUFFER (utf-8) {:?}", line),
                    Err(_) => log::trace!("CURRENT BUFFER (binary) {:?}", self.buffer.as_slice()),
                }
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
                .windows(LINE_END.len())
                .position(|slice| slice == LINE_END.as_bytes())
            {
                // If we see a line break, the modem has probably sent us a message

                let line_end = position + LINE_END.len();
                let Ok(line) = from_utf8(&self.buffer[..position]) else {
                    self.buffer.rotate_left(line_end);
                    self.buffer.truncate(self.buffer.len() - line_end);
                    return Err(Error::InvalidUtf8);
                };
                log::trace!("RECV LINE: {:?}", line);

                // Ignore empty lines, as well as echoed lines (which end with \r\r\n)
                if line.trim().is_empty() || line.ends_with("\r\r") {
                    self.buffer.rotate_left(line_end);
                    self.buffer.truncate(self.buffer.len() - line_end);

                    continue;
                }

                let line = line.trim(); // The modem likes to be inconsistent with white space
                let line = heapless::String::from(line);

                // Remove the line from the buffer
                self.buffer.rotate_left(line_end);
                self.buffer.truncate(self.buffer.len() - (line_end));

                return Ok(line);
            }

            if self.buffer.capacity() == self.buffer.len() {
                panic!(
                    "read buffer is full, this should never happen. contents: {:?}",
                    self.buffer.as_slice()
                );
            }

            let mut buf = [0u8; 256];
            let amount = Read::read(
                &mut self.read,
                &mut buf[..self.buffer.capacity() - self.buffer.len()],
            )
            .await
            .map_err(|_| Error::Serial)?;

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
