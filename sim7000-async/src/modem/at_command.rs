use core::num::{ParseFloatError, ParseIntError};

pub mod unsolicited;

pub(crate) struct ATParseErr;
pub(crate) trait ATParseLine: Sized {
    fn from_line(line: &str) -> Result<Self, ATParseErr>;
}

impl From<ParseIntError> for ATParseErr {
    fn from(_: ParseIntError) -> Self {
        ATParseErr
    }
}

impl From<ParseFloatError> for ATParseErr {
    fn from(_: ParseFloatError) -> Self {
        ATParseErr
    }
}

/// Stub AT response parser that just checks if the line starts with `prefix`
fn stub_parser_prefix<T>(line: &str, prefix: &'static str, t: T) -> Result<T, ATParseErr> {
    line.starts_with(prefix).then(|| t).ok_or(ATParseErr)
}
