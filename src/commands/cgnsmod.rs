use crate::Error;

use super::{AtCommand, AtDecode, AtEncode, AtRead};

pub struct Cgnsmod;

impl AtCommand for Cgnsmod {
    const COMMAND: &'static str = "AT+CGNSMOD";
}

impl AtRead for Cgnsmod {
    type Output = ();
}

pub struct CgnsmodParam {
    pub enable_glonass: bool,
    pub enable_beidou: bool,
    pub enable_galilean: bool,
}

impl AtEncode for CgnsmodParam {
    fn encode<B: crate::SerialWrite>(
        &self,
        encoder: &mut super::Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(1)?;
        encoder.encode_str(",")?;
        encoder.encode_scalar(self.enable_glonass as i32)?;
        encoder.encode_str(",")?;
        encoder.encode_scalar(self.enable_beidou as i32)?;
        encoder.encode_str(",")?;
        encoder.encode_scalar(self.enable_galilean as i32)
    }
}

impl AtDecode for CgnsmodParam {
    fn decode<B: crate::SerialReadTimeout>(
        decoder: &mut super::Decoder<B>,
        timeout: embedded_time::duration::Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        let _ = decoder.decode_scalar(timeout)?;
        decoder.expect_str(",", timeout)?;
        let enable_glonass = decoder.decode_scalar(timeout)? == 1;
        decoder.expect_str(",", timeout)?;
        let enable_beidou = decoder.decode_scalar(timeout)? == 1;
        decoder.expect_str(",", timeout)?;
        let enable_galilean = decoder.decode_scalar(timeout)? == 1;

        Ok(CgnsmodParam {
            enable_glonass,
            enable_beidou,
            enable_galilean,
        })
    }
}
