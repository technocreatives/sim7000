use crate::{Error, SerialReadTimeout, SerialWrite, drain_relay};
use embedded_time::duration::Milliseconds;

mod at;
mod ate;
mod ccid;
mod cfgri;
mod cgmr;
mod cgnsinf;
mod cgnspwr;
mod cgreg;
mod ciicr;
mod ciprxget;
mod cipsend;
mod cipshut;
mod cipstart;
mod cipstatus;
mod cmee;
mod cmnb;
mod cnmp;
mod cops;
mod cpsi;
mod csclk;
mod csq;
mod cstt;
mod ifc;
mod ipr;

pub use at::*;
pub use ate::*;
pub use ccid::*;
pub use cfgri::*;
pub use cgmr::*;
pub use cgnsinf::*;
pub use cgnspwr::*;
pub use cgreg::*;
pub use ciicr::*;
pub use ciprxget::*;
pub use cipsend::*;
pub use cipshut::*;
pub use cipstart::*;
pub use cipstatus::*;
pub use cmee::*;
pub use cmnb::*;
pub use cnmp::*;
pub use cops::*;
pub use cpsi::*;
pub use csclk::*;
pub use csq::*;
pub use cstt::*;
pub use ifc::*;
pub use ipr::*;

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
        timeout: Milliseconds,
    ) -> Result<Self::Output, Error<B::SerialError>> {
        drain_relay(serial, Milliseconds(0))?;

        let mut encoder = Encoder::new(serial);
        encoder.encode_str(<Self as AtCommand>::COMMAND)?;
        encoder.encode_str("=")?;

        parameter.encode(&mut encoder)?;

        // Wait 200ms for an echo to appear.
        let echoed = drain_relay(serial, Milliseconds(200))?;

        serial.write(b"\r")?;

        let mut decoder = Decoder::new(serial);

        // Drain the echoed newline
        if echoed {
            decoder.expect_empty(timeout)?;
            decoder.end_line();
        }

        // Drain the newline that starts every command
        decoder.expect_empty(timeout)?;
        decoder.end_line();

        Self::Output::decode(&mut decoder, timeout)
    }
}

pub trait AtRead: AtCommand {
    type Output: AtDecode;

    fn read<B: SerialReadTimeout + SerialWrite>(
        &self,
        serial: &mut B,
        timeout: Milliseconds,
    ) -> Result<Self::Output, Error<B::SerialError>> {
        drain_relay(serial, Milliseconds(0))?;

        let mut encoder = Encoder::new(serial);
        encoder.encode_str(<Self as AtCommand>::COMMAND)?;
        encoder.encode_str("?")?;

        // Echo may or may not be enabled. The following code deals with a potential echo.

        // Wait 200ms for an echo to appear.
        let echoed = drain_relay(serial, Milliseconds(200))?;

        serial.write(b"\r")?;

        let mut decoder = Decoder::new(serial);

        // Drain the echoed newline
        if echoed {
            decoder.expect_empty(timeout)?;
            decoder.end_line();
        }

        // Drain the newline that starts every command
        decoder.expect_empty(timeout)?;
        decoder.end_line();

        Self::Output::decode(&mut decoder, timeout)
    }
}

pub trait AtExecute: AtCommand {
    type Output: AtDecode;

    fn execute<B: SerialReadTimeout + SerialWrite>(
        &self,
        serial: &mut B,
        timeout: Milliseconds,
    ) -> Result<Self::Output, Error<B::SerialError>> {
        drain_relay(serial, Milliseconds(0))?;

        let mut encoder = Encoder::new(serial);
        encoder.encode_str(<Self as AtCommand>::COMMAND)?;

        // Echo may or may not be enabled. The following code deals with a potential echo.

        // Wait 200ms for an echo to appear.
        let echoed = drain_relay(serial, Milliseconds(200))?;

        serial.write(b"\r")?;

        let mut decoder = Decoder::new(serial);

        // Drain the echoed newline
        if echoed {
            decoder.expect_empty(timeout)?;
            decoder.end_line();
        }

        // Drain the newline that starts every command
        decoder.expect_empty(timeout)?;
        decoder.end_line();

        Self::Output::decode(&mut decoder, timeout)
    }
}

pub trait AtEncode {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>) -> Result<(), Error<B::SerialError>>;
}

impl<'a> AtEncode for &'a [u8] {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>) -> Result<(), Error<B::SerialError>> {
        encoder.encode_bytes(self)
    }
}

impl AtEncode for i32 {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self)
    }
}

pub trait AtDecode: Sized {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>>;
}

impl AtDecode for () {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("OK", timeout)
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
        log::trace!("at_debug: SEND STR: {:?}", value);

        let data = value.as_bytes();
        self.encode_bytes(data)
    }

    pub fn encode_scalar(&mut self, value: i32) -> Result<(), Error<B::SerialError>> {
        let string: heapless::String<11> = heapless::String::from(value);
        self.encode_str(&string)
    }
}

pub struct Decoder<'a, B: SerialReadTimeout> {
    buf: &'a mut B,
    current_line: Option<heapless::String<256>>,
    offset: usize,
}

impl<'a, B: SerialReadTimeout> Decoder<'a, B> {
    pub fn new(buf: &'a mut B) -> Self {
        Self {
            buf,
            current_line: None,
            offset: 0,
        }
    }

    pub fn expect_empty(&mut self, timeout: Milliseconds) -> Result<(), Error<B::SerialError>> {
        self.fill_line(timeout)?;

        if !&self.current_line.as_ref().unwrap()[self.offset..].is_empty() {
            return Err(crate::Error::DecodingFailed);
        }

        Ok(())
    }

    pub fn decode_scalar(&mut self, timeout: Milliseconds) -> Result<i32, Error<B::SerialError>> {
        self.fill_line(timeout)?;
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
        timeout: Milliseconds,
    ) -> Result<(), Error<B::SerialError>> {
        self.fill_line(timeout)?;
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

    pub fn remainder_str(&mut self, timeout: Milliseconds) -> Result<&str, Error<B::SerialError>> {
        self.fill_line(timeout)?;
        Ok(&self.current_line.as_ref().unwrap()[self.offset..])
    }

    fn fill_line(&mut self, timeout: Milliseconds) -> Result<(), Error<B::SerialError>> {
        if self.current_line.is_none() {
            let mut buf = [0u8; 256];
            let line = self
                .buf
                .read_line(&mut buf, timeout)?
                .ok_or(crate::Error::Timeout)?;

            #[cfg(feature = "at_debug")]
            log::info!("at_debug: RECV LINE: {:?}", line);

            self.current_line = Some(heapless::String::from(line));
            self.offset = 0;
        }

        Ok(())
    }

    fn read_exact(
        &mut self,
        buf: &mut [u8],
        timeout: Milliseconds,
    ) -> Result<(), Error<B::SerialError>> {
        if self.buf.read_exact(buf, timeout)?.is_none() {
            return Err(crate::Error::Timeout);
        }

        Ok(())
    }
}
