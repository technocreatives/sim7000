use super::{AtParseErr, AtParseLine, AtResponse, ResponseCode};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GenericOk;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SimError {
    /// Generic error
    Generic,

    /// Error relating to mobile equipment or to the network.
    CmeErr { code: u32 },

    /// Error relating to message service or to the network.
    CmsErr { code: u32 },
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WritePrompt;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CloseOk {
    pub connection: usize,
}

impl AtParseLine for GenericOk {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        // TODO: SHUT OK should be seperate type
        (line == "OK" || line == "SHUT OK")
            .then(|| GenericOk)
            .ok_or_else(|| "Not 'OK'".into())
    }
}

impl AtResponse for GenericOk {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::Ok(ok) => Ok(ok),
            _ => Err(code),
        }
    }
}

impl AtParseLine for SimError {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        if let Some(code) = line.strip_prefix("+CME ERROR: ") {
            Ok(SimError::CmeErr {
                code: code.parse()?,
            })
        } else if let Some(code) = line.strip_prefix("+CMS ERROR: ") {
            Ok(SimError::CmsErr {
                code: code.parse()?,
            })
        } else if line == "ERROR" {
            Ok(SimError::Generic)
        } else {
            Err("Not a valid error code".into())
        }
    }
}

impl AtParseLine for WritePrompt {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        line.eq("> ")
            .then(|| WritePrompt)
            .ok_or_else(|| "Not '> '".into())
    }
}

impl AtResponse for WritePrompt {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::WritePrompt(prompt) => Ok(prompt),
            _ => Err(code),
        }
    }
}

impl AtParseLine for CloseOk {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let connection = line
            .strip_suffix(", CLOSE OK")
            .ok_or("Missing ', CLOSE OK'")?
            .parse()?;

        Ok(CloseOk { connection })
    }
}

impl AtResponse for CloseOk {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::CloseOk(close_ok) => Ok(close_ok),
            _ => Err(code),
        }
    }
}
