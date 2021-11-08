use super::{AtCommand, AtEncode, AtWrite};

pub struct Sapbr;

impl AtCommand for Sapbr {
    const COMMAND: &'static str = "AT+SAPBR";
}

impl<'a> AtWrite<'a> for Sapbr {
    type Input = BearerMode<'a>;

    type Output = ();
}

pub enum BearerMode<'a> {
    Close(u8),
    Open(u8),
    SetParameter(u8, BearerParameter<'a>),
}

pub enum BearerParameter<'a> {
    Apn(&'a str),
    Username(&'a str),
    Password(&'a str),
}

impl AtEncode for BearerMode<'_> {
    fn encode<B: crate::SerialWrite>(
        &self,
        encoder: &mut super::Encoder<B>,
    ) -> Result<(), crate::Error<B::SerialError>> {
        match self {
            BearerMode::Close(cid) => {
                encoder.encode_scalar(0)?;
                encoder.encode_str(",")?;
                encoder.encode_scalar(*cid as i32)?;
            }
            BearerMode::Open(cid) => {
                encoder.encode_scalar(1)?;
                encoder.encode_str(",")?;
                encoder.encode_scalar(*cid as i32)?;
            }
            BearerMode::SetParameter(cid, param) => {
                let (name, value) = match param {
                    BearerParameter::Apn(apn) => ("APN", apn),
                    BearerParameter::Username(user) => ("USER", user),
                    BearerParameter::Password(pass) => ("PWD", pass),
                };

                encoder.encode_scalar(3)?;
                encoder.encode_str(",")?;
                encoder.encode_scalar(*cid as i32)?;
                encoder.encode_str(",")?;
                encoder.encode_str("\"")?;
                encoder.encode_str(name)?;
                encoder.encode_str("\"")?;
                encoder.encode_str(",")?;
                encoder.encode_str("\"")?;
                encoder.encode_str(value)?;
                encoder.encode_str("\"")?;
            }
        }

        Ok(())
    }
}
