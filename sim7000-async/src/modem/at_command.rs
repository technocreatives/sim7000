use core::num::{ParseFloatError, ParseIntError};

pub mod unsolicited;

#[derive(Clone, Copy, Default, Debug)]
pub(crate) struct ATParseErr {
    #[allow(dead_code)]
    message: &'static str,
}

pub(crate) trait ATParseLine: Sized {
    fn from_line(line: &str) -> Result<Self, ATParseErr>;
}

impl From<&'static str> for ATParseErr {
    fn from(message: &'static str) -> Self {
        ATParseErr { message }
    }
}

impl From<ParseIntError> for ATParseErr {
    fn from(_: ParseIntError) -> Self {
        ATParseErr {
            message: "Failed to parse integer",
        }
    }
}

impl From<ParseFloatError> for ATParseErr {
    fn from(_: ParseFloatError) -> Self {
        ATParseErr {
            message: "Failed to parse float",
        }
    }
}

/// Stub AT response parser that just checks if the line starts with `prefix`
fn stub_parser_prefix<T>(line: &str, prefix: &'static str, t: T) -> Result<T, ATParseErr> {
    line.starts_with(prefix).then(|| t).ok_or(ATParseErr {
        message: "Stub parser: Missing prefix",
    })
}
