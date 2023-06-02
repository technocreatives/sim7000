use crate::Error;

use super::{AtCommand, AtEncode, AtWrite};

pub struct Cedrxs;

impl AtCommand for Cedrxs {
    const COMMAND: &'static str = "AT+CEDRXS";
}

impl AtWrite<'_> for Cedrxs {
    type Input = EdrxParams;
    type Output = ();
}

#[repr(i32)]
#[derive(Clone, Copy, Debug)]
pub enum EdrxMode {
    Disable = 0,
    Enable = 1,
    AutoReport = 2,
}

impl AtEncode for EdrxMode {
    fn encode<B: crate::SerialWrite>(
        &self,
        encoder: &mut super::Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self as i32)
    }
}

#[repr(i32)]
#[derive(Clone, Copy, Debug)]
pub enum EdrxBandMode {
    CatM = 4,
    NbIot = 5,
}

impl AtEncode for EdrxBandMode {
    fn encode<B: crate::SerialWrite>(
        &self,
        encoder: &mut super::Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self as i32)
    }
}

#[repr(i32)]
#[derive(Clone, Copy, Debug)]
pub enum EdrxInterval {
    S5_12,
    S10_24,
    S20_48,
    S40_96,
    S61_44,
    S81_92,
    S102_40,
    S122_88,
    S143_36,
    S163_84,
    S327_68,
    S655_36,
    S1310_72,
    S2621_44,
    S5242_88,
    S10485_76,
}

impl AtEncode for EdrxInterval {
    fn encode<B: crate::SerialWrite>(
        &self,
        encoder: &mut super::Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        match self {
            EdrxInterval::S5_12 => encoder.encode_str("\"0000\""),
            EdrxInterval::S10_24 => encoder.encode_str("\"0001\""),
            EdrxInterval::S20_48 => encoder.encode_str("\"0010\""),
            EdrxInterval::S40_96 => encoder.encode_str("\"0011\""),
            EdrxInterval::S61_44 => encoder.encode_str("\"0100\""),
            EdrxInterval::S81_92 => encoder.encode_str("\"0101\""),
            EdrxInterval::S102_40 => encoder.encode_str("\"0110\""),
            EdrxInterval::S122_88 => encoder.encode_str("\"0111\""),
            EdrxInterval::S143_36 => encoder.encode_str("\"1000\""),
            EdrxInterval::S163_84 => encoder.encode_str("\"1001\""),
            EdrxInterval::S327_68 => encoder.encode_str("\"1010\""),
            EdrxInterval::S655_36 => encoder.encode_str("\"1011\""),
            EdrxInterval::S1310_72 => encoder.encode_str("\"1100\""),
            EdrxInterval::S2621_44 => encoder.encode_str("\"1101\""),
            EdrxInterval::S5242_88 => encoder.encode_str("\"1110\""),
            EdrxInterval::S10485_76 => encoder.encode_str("\"1111\""),
        }
    }
}

pub struct EdrxParams {
    pub mode: EdrxMode,
    pub band: EdrxBandMode,
    pub interval: EdrxInterval,
}

impl AtEncode for EdrxParams {
    fn encode<B: crate::SerialWrite>(
        &self,
        encoder: &mut super::Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        self.mode.encode(encoder)?;
        encoder.encode_str(",")?;
        self.band.encode(encoder)?;
        encoder.encode_str(",")?;
        self.interval.encode(encoder)
    }
}
