use crate::{drain_relay, Error, SerialReadTimeout, SerialWrite};

mod at;
mod ate;
mod ccid;
mod cedrxs;
mod cfgri;
mod cgmr;
mod cgnscold;
mod cgnscpy;
mod cgnsinf;
mod cgnsmod;
mod cgnspwr;
mod cgnsxtra;
mod cgreg;
mod ciicr;
mod ciprxget;
mod cipsend;
mod cipshut;
mod cipstart;
mod cipstatus;
mod cmee;
mod cmnb;
mod cnact;
mod cnmp;
mod cntp;
mod cntpcid;
mod cops;
mod cpsi;
mod csclk;
mod csq;
mod cstt;
mod httptofs;
mod ifc;
mod ipr;
mod sapbr;

pub use at::*;
pub use ate::*;
pub use ccid::*;
pub use cedrxs::*;
pub use cfgri::*;
pub use cgmr::*;
pub use cgnsinf::*;
pub use cgnspwr::*;
pub use cgnsxtra::*;
pub use cgreg::*;
pub use ciicr::*;
pub use ciprxget::*;
pub use cipsend::*;
pub use cipshut::*;
pub use cipstart::*;
pub use cipstatus::*;
pub use cmee::*;
pub use cmnb::*;
pub use cnact::*;
pub use cnmp::*;
pub use cntp::*;
pub use cntpcid::*;
pub use cops::*;
pub use cpsi::*;
pub use csclk::*;
pub use csq::*;
pub use cstt::*;
pub use httptofs::*;
pub use ifc::*;
pub use ipr::*;
pub use sapbr::*;

pub trait AtCommand {
    const COMMAND: &'static str;
}

pub trait AtWrite<'a>: AtCommand {
    type Input: AtEncode;
    type Output: AtDecode;

    fn write<B: SerialReadTimeout + SerialWrite>(
        &self,
        parameter: Self::Input,
        serial: &mut B,
        timeout_ms: u32,
    ) -> Result<Self::Output, Error<B::SerialError>> {
        drain_relay(serial, 0)?;

        let mut encoder = Encoder::new(serial);
        encoder.encode_str(<Self as AtCommand>::COMMAND)?;
        encoder.encode_str("=")?;

        parameter.encode(&mut encoder)?;

        serial.write(b"\r")?;

        let mut decoder = Decoder::new(serial);

        Self::Output::decode(&mut decoder, timeout_ms)
    }
}

pub trait AtRead: AtCommand {
    type Output: AtDecode;

    fn read<B: SerialReadTimeout + SerialWrite>(
        &self,
        serial: &mut B,
        timeout_ms: u32,
    ) -> Result<Self::Output, Error<B::SerialError>> {
        drain_relay(serial, 0)?;

        let mut encoder = Encoder::new(serial);
        encoder.encode_str(<Self as AtCommand>::COMMAND)?;
        encoder.encode_str("?")?;

        serial.write(b"\r")?;

        let mut decoder = Decoder::new(serial);

        Self::Output::decode(&mut decoder, timeout_ms)
    }
}

pub trait AtExecute: AtCommand {
    type Output: AtDecode;

    fn execute<B: SerialReadTimeout + SerialWrite>(
        &self,
        serial: &mut B,
        timeout_ms: u32,
    ) -> Result<Self::Output, Error<B::SerialError>> {
        drain_relay(serial, 0)?;

        let mut encoder = Encoder::new(serial);
        encoder.encode_str(<Self as AtCommand>::COMMAND)?;

        serial.write(b"\r")?;

        let mut decoder = Decoder::new(serial);

        Self::Output::decode(&mut decoder, timeout_ms)
    }
}

pub trait AtEncode {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>)
        -> Result<(), Error<B::SerialError>>;
}

impl<'a> AtEncode for &'a [u8] {
    fn encode<B: SerialWrite>(
        &self,
        encoder: &mut Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        encoder.encode_bytes(self)
    }
}

impl AtEncode for i32 {
    fn encode<B: SerialWrite>(
        &self,
        encoder: &mut Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self)
    }
}

pub trait AtDecode: Sized {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout_ms: u32,
    ) -> Result<Self, Error<B::SerialError>>;
}

impl AtDecode for () {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout_ms: u32,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("OK", timeout_ms)
    }
}

pub struct Encoder<'a, B: SerialWrite> {
    buf: &'a mut B,
}

impl<'a, B: SerialWrite> Encoder<'a, B> {
    pub fn new(buf: &'a mut B) -> Self {
        Self { buf }
    }

    pub fn encode_bytes(&mut self, data: &[u8]) -> Result<(), Error<B::SerialError>> {
        self.buf.write(data).map_err(Into::into)
    }

    pub fn encode_str(&mut self, value: &str) -> Result<(), Error<B::SerialError>> {
        log::trace!("SEND STR: {:?}", value);

        let data = value.as_bytes();
        self.encode_bytes(data)
    }

    pub fn encode_scalar(&mut self, value: i32) -> Result<(), Error<B::SerialError>> {
        let string: heapless::String<11> = heapless::String::from(value);
        self.encode_str(&string)
    }
}

pub struct Decoder<'a, B: SerialReadTimeout> {
    read: &'a mut B,
    buffer: heapless::Vec<u8, 256>,
    current_line: Option<heapless::String<256>>,
    offset: usize,
}

impl<'a, B: SerialReadTimeout> Decoder<'a, B> {
    pub fn new(read: &'a mut B) -> Self {
        Self {
            buffer: heapless::Vec::new(),
            read,
            current_line: None,
            offset: 0,
        }
    }

    pub fn decode_scalar(&mut self, timeout_ms: u32) -> Result<i32, Error<B::SerialError>> {
        self.fill_line(timeout_ms)?;
        let line = &self.current_line.as_ref().unwrap()[self.offset..];

        let index = line
            .find(|ch| !('0'..='9').contains(&ch) && ch != '-')
            .unwrap_or_else(|| line.len());

        if index == 0 {
            return Err(crate::Error::DecodingFailed);
        }

        self.offset += index;

        let num = line.split_at(index).0;

        num.parse().map_err(|_| crate::Error::DecodingFailed)
    }

    pub fn end_line(&mut self) {
        self.current_line = None;
    }

    pub fn expect_str(
        &mut self,
        value: &str,
        timeout_ms: u32,
    ) -> Result<(), Error<B::SerialError>> {
        self.fill_line(timeout_ms)?;
        let line = &self.current_line.as_ref().unwrap()[self.offset..];
        if line.len() < value.len() {
            return Err(crate::Error::DecodingFailed);
        }

        if !line.starts_with(value) {
            return Err(crate::Error::DecodingFailed);
        }

        self.offset += value.len();
        Ok(())
    }

    pub fn remainder_str(&mut self, timeout_ms: u32) -> Result<&str, Error<B::SerialError>> {
        self.fill_line(timeout_ms)?;
        Ok(&self.current_line.as_ref().unwrap()[self.offset..])
    }

    fn fill_line(&mut self, timeout_ms: u32) -> Result<(), Error<B::SerialError>> {
        if self.current_line.is_none() {
            loop {
                log::trace!("CURRENT BUFFER {:?}", core::str::from_utf8(&self.buffer));
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
                        log::trace!("RECV LINE: {:?}", s);
                        self.current_line = Some(heapless::String::from(s));
                        self.offset = 0;

                        self.buffer.rotate_left(line_end + 2);
                        self.buffer.truncate(self.buffer.len() - (line_end + 2));

                        if !bad_codes().contains(self.current_line.as_ref().unwrap().as_str()) {
                            return Ok(());
                        }
                    }
                }

                let mut buf = [0u8; 256];
                if let Some(amount) = self.read.read(
                    &mut buf[..self.buffer.capacity() - self.buffer.len()],
                    timeout_ms,
                )? {
                    self.buffer
                        .extend_from_slice(&buf[..amount])
                        .map_err(|_| Error::BufferOverflow)?;
                } else {
                    return Err(Error::Timeout);
                }
            }
        }

        Ok(())
    }

    fn read_exact(&mut self, buf: &mut [u8], timeout_ms: u32) -> Result<(), Error<B::SerialError>> {
        if !self.buffer.is_empty() {
            let len = core::cmp::min(self.buffer.len(), buf.len());
            buf[..len].copy_from_slice(&self.buffer[..len]);
            if len < buf.len() && self.read.read_exact(&mut buf[len..], timeout_ms)?.is_none() {
                return Err(crate::Error::Timeout);
            }
        } else if self.read.read_exact(buf, timeout_ms)?.is_none() {
            return Err(crate::Error::Timeout);
        }

        Ok(())
    }
}

static BAD_CODES: spin::Once<heapless::FnvIndexSet<&'static str, 16>> = spin::Once::new();

fn bad_codes() -> &'static heapless::FnvIndexSet<&'static str, 16> {
    BAD_CODES.call_once(|| {
        let mut m = heapless::FnvIndexSet::new();

        // insert can only fail when there is no capacity left
        m.insert("CLOSED").unwrap();
        m.insert("RDY").unwrap();
        m
    })
}
