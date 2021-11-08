use super::{AtCommand, AtDecode, AtEncode, AtWrite};

pub struct Httptofs;

impl AtCommand for Httptofs {
    const COMMAND: &'static str = "AT+HTTPTOFS";
}

impl<'a> AtWrite<'a> for Httptofs {
    type Input = HttpParams<'a, 'a>;

    type Output = HttpResponse;
}

pub struct HttpParams<'a, 'b> {
    url: &'a str,
    path: &'b str,
}

impl AtEncode for HttpParams<'_, '_> {
    fn encode<B: crate::SerialWrite>(
        &self,
        encoder: &mut super::Encoder<B>,
    ) -> Result<(), crate::Error<B::SerialError>> {
        encoder.encode_str("\"")?;
        encoder.encode_str(self.url)?;
        encoder.encode_str("\",")?;
        encoder.encode_str(self.path)?;
        encoder.encode_str("\"")
    }
}

pub struct HttpResponse {
    pub status: u16,
    pub len: u32,
}

impl AtDecode for HttpResponse {
    fn decode<B: crate::SerialReadTimeout>(
        decoder: &mut super::Decoder<B>,
        timeout: embedded_time::duration::Milliseconds,
    ) -> Result<Self, crate::Error<B::SerialError>> {
        decoder.expect_str("+HTTPTOFS: ", timeout)?;
        let status = decoder.decode_scalar(timeout)? as u16;
        decoder.expect_str(",", timeout)?;
        let len = decoder.decode_scalar(timeout)? as u32;

        Ok(HttpResponse { status, len })
    }
}
