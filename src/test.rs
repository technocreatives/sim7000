use std::collections::VecDeque;

use embedded_time::duration::Milliseconds;

use crate::{Serial, SerialReadTimeout, SerialWrite};

pub struct MockSerial {
    operations: VecDeque<SerialOperation>,
}

enum SerialOperation {
    Read(Vec<u8>),
    Write(Vec<u8>),
}

pub struct MockSerialBuilder {
    mock: MockSerial,
}

impl MockSerialBuilder {
    pub fn expect_read(mut self, bytes: &[u8]) -> MockSerialBuilder {
        self.mock
            .operations
            .push_back(SerialOperation::Read(Vec::from(bytes)));
        self
    }

    pub fn expect_write(mut self, bytes: &[u8]) -> MockSerialBuilder {
        self.mock
            .operations
            .push_back(SerialOperation::Write(Vec::from(bytes)));
        self
    }

    pub fn finalize(self) -> MockSerial {
        self.mock
    }
}

impl MockSerial {
    pub fn build() -> MockSerialBuilder {
        MockSerialBuilder {
            mock: MockSerial {
                operations: VecDeque::new(),
            },
        }
    }
}

impl Serial for MockSerial {
    type SerialError = ();
}

impl SerialReadTimeout for MockSerial {
    fn read_exact(
        &mut self,
        buf: &mut [u8],
        timeout: embedded_time::duration::Milliseconds,
    ) -> Result<Option<()>, Self::SerialError> {
        // hack for draining echoes
        if timeout <= Milliseconds(200u32) {
            return Ok(None);
        }

        match self.operations.front_mut() {
            Some(SerialOperation::Read(bytes)) => {
                buf.copy_from_slice(&bytes[..buf.len()]);
                *bytes = Vec::from(&bytes[buf.len()..]);

                if bytes.len() == 0 {
                    self.operations.pop_front();
                }

                Ok(Some(()))
            }
            Some(SerialOperation::Write(bytes)) => panic!(
                "Expected Write of {:?}, read called instead",
                bytes.as_slice()
            ),
            None => Ok(None),
        }
    }

    fn read_line<'a>(
        &mut self,
        out: &'a mut [u8],
        _: embedded_time::duration::Milliseconds,
    ) -> Result<Option<&'a str>, Self::SerialError> {
        match self.operations.pop_front() {
            Some(SerialOperation::Read(bytes)) => {
                (out[..bytes.len()]).copy_from_slice(&bytes);
                Ok(Some(core::str::from_utf8(&out[..bytes.len()]).unwrap()))
            }
            Some(SerialOperation::Write(bytes)) => panic!(
                "Expected Write of {:?}, read called instead",
                bytes.as_slice()
            ),
            None => Ok(None),
        }
    }
}

impl SerialWrite for MockSerial {
    fn write(&mut self, mut buf: &[u8]) -> Result<(), Self::SerialError> {
        loop {
            match self.operations.front_mut() {
                Some(SerialOperation::Read(bytes)) => panic!(
                    "Expected Read of {:?}, write called instead with {:?}",
                    bytes.as_slice(),
                    buf
                ),
                Some(SerialOperation::Write(bytes)) => {
                    if bytes.len() < buf.len() {
                        assert_eq!(&buf[..bytes.len()], bytes);
                        assert_eq!(buf[bytes.len()], b'\r');
                        buf = &buf[bytes.len() + 1..];
                        self.operations.pop_front();

                        if buf.len() == 0 {
                            return Ok(());
                        }

                        continue;
                    } else if bytes.len() > buf.len() {
                        assert_eq!(buf, &bytes[..buf.len()]);
                        *bytes = Vec::from(&bytes[buf.len()..]);
                        return Ok(());
                    } else if bytes.len() == buf.len() {
                        assert_eq!(buf, bytes);
                        self.operations.pop_front();
                        return Ok(());
                    }
                }
                None => panic!("Expected no more operations, write called instead"),
            }
        }
    }
}
