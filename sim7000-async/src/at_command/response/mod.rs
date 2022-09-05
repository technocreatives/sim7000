use super::{ATParseErr, ATParseLine};

pub mod ccid;
pub mod cifsrex;
pub mod csq;

pub use ccid::Iccid;
pub use cifsrex::IpExt;
pub use csq::SignalQuality;

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

pub trait ATResponse: Sized {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode>;
}

impl ATParseLine for GenericOk {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        // TODO: SHUT OK should be seperate type
        (line == "OK" || line == "SHUT OK")
            .then(|| GenericOk)
            .ok_or_else(|| "Not 'OK'".into())
    }
}

impl ATResponse for GenericOk {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::Ok(ok) => Ok(ok),
            _ => Err(code),
        }
    }
}

impl ATParseLine for SimError {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        if let Some(code) = line.strip_prefix("+CME ERROR") {
            Ok(SimError::CmeErr {
                code: code.parse()?,
            })
        } else if let Some(code) = line.strip_prefix("+CMS ERROR") {
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

impl ATParseLine for WritePrompt {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        line.eq("> ")
            .then(|| WritePrompt)
            .ok_or_else(|| "Not '> '".into())
    }
}

impl ATResponse for WritePrompt {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::WritePrompt(prompt) => Ok(prompt),
            _ => Err(code),
        }
    }
}

impl ATParseLine for CloseOk {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let connection = line
            .strip_suffix(", CLOSE OK")
            .ok_or("Missing ', CLOSE OK'")?
            .parse()?;

        Ok(CloseOk { connection })
    }
}

impl ATResponse for CloseOk {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::CloseOk(close_ok) => Ok(close_ok),
            _ => Err(code),
        }
    }
}

/// Sim7000 AT-command response code
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ResponseCode {
    Ok(GenericOk),
    Error(SimError),
    WritePrompt(WritePrompt), // "> "
    CloseOk(CloseOk),
    IpExt(IpExt),
    Iccid(Iccid),
    SignalQuality(SignalQuality),
}

impl ATParseLine for ResponseCode {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        /// Create a function that tries to parse the line into an Urc::T
        fn parse<'a, T: ATParseLine>(
            line: &'a str,
            f: impl Fn(T) -> ResponseCode + 'a,
        ) -> impl Fn(ATParseErr) -> Result<ResponseCode, ATParseErr> + 'a {
            move |_| Ok(f(T::from_line(line)?))
        }

        Err(ATParseErr::default())
            .or_else(parse(line, ResponseCode::Ok))
            .or_else(parse(line, ResponseCode::Error))
            .or_else(parse(line, ResponseCode::WritePrompt))
            .or_else(parse(line, ResponseCode::CloseOk))
            .or_else(parse(line, ResponseCode::IpExt))
            .or_else(parse(line, ResponseCode::Iccid))
            .or_else(parse(line, ResponseCode::SignalQuality))
            .map_err(|_| "Unknown response code".into())
    }
}
