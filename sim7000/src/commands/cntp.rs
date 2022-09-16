use crate::Error;

use super::{AtCommand, AtDecode, AtEncode, AtExecute, AtWrite};

pub struct Cntp;

impl AtCommand for Cntp {
    const COMMAND: &'static str = "AT+CNTP";
}

impl<'a> AtWrite<'a> for Cntp {
    type Input = CntpParams<'a>;

    type Output = ();
}

impl AtExecute for Cntp {
    type Output = CntpResponse;
}

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum CntpMode {
    SetNetwork = 0,
    OutputTime = 1,
    SetAndOutput = 2,
}

pub struct CntpParams<'a> {
    pub server: &'a str,
    /// Timezone ranges from -47 to 48. Each whole number represents a 15 minute offset. 0 is UTC+0.
    pub timezone: i32,
    pub cid: i32,
    pub mode: CntpMode,
}

impl AtEncode for CntpParams<'_> {
    fn encode<B: crate::SerialWrite>(
        &self,
        encoder: &mut super::Encoder<B>,
    ) -> Result<(), crate::Error<B::SerialError>> {
        encoder.encode_str("\"")?;
        encoder.encode_str(self.server)?;
        encoder.encode_str("\"")?;
        encoder.encode_str(",")?;
        encoder.encode_scalar(self.timezone)?;
        encoder.encode_str(",")?;
        encoder.encode_scalar(self.cid)?;
        encoder.encode_str(",")?;
        encoder.encode_scalar(self.mode as i32)
    }
}

#[derive(Debug)]
pub enum CntpResponse {
    Success(heapless::String<256>),
    NetworkError,
    DnsError,
    ConnectionError,
    ServiceError,
    ServiceTimeout,
}

impl AtDecode for CntpResponse {
    fn decode<B: crate::SerialReadTimeout>(
        decoder: &mut super::Decoder<B>,
        timeout_ms: u32,
    ) -> Result<Self, crate::Error<B::SerialError>> {
        decoder.expect_str("OK", timeout_ms)?;
        decoder.end_line();

        Ok(match decoder.decode_scalar(timeout_ms)? {
            1 => {
                decoder.expect_str(",", timeout_ms)?;
                let time = decoder.remainder_str(timeout_ms)?.into();
                CntpResponse::Success(time)
            }
            61 => CntpResponse::NetworkError,
            62 => CntpResponse::DnsError,
            63 => CntpResponse::ConnectionError,
            64 => CntpResponse::ServiceError,
            65 => CntpResponse::ServiceTimeout,
            _ => return Err(Error::DecodingFailed),
        })
    }
}
