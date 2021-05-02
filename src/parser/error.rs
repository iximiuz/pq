use nom;

#[derive(Debug, PartialEq)]
pub struct ParseError {
    message: String,
    offset: usize,
    line: u32,
}

impl ParseError {
    pub fn new(message: String, offset: usize, line: u32) -> Self {
        Self {
            message,
            offset,
            line,
        }
    }
}

impl<I: std::fmt::Display> nom::error::ParseError<I> for ParseError {
    fn from_error_kind(input: I, kind: nom::error::ErrorKind) -> Self {
        Self::new(
            format!("from_error_kind input={} kind={:#?}", input, kind),
            0,
            0,
        )
    }

    fn append(_input: I, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }

    fn from_char(input: I, c: char) -> Self {
        Self::new(format!("from_char input={} char={}", input, c), 0, 0)
    }
}

impl From<ParseError> for String {
    fn from(err: ParseError) -> Self {
        format!("{}:{} {}", err.line, err.offset, err.message)
    }
}

impl From<nom::Err<ParseError>> for ParseError {
    fn from(err: nom::Err<ParseError>) -> Self {
        match err {
            nom::Err::Error(e) | nom::Err::Failure(e) => e,
            nom::Err::Incomplete(_) => unreachable!(),
        }
    }
}
