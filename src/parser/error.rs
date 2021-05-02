use nom::error::{ErrorKind, ParseError};

#[derive(Debug, PartialEq)]
pub struct MyError {
    pub message: String,
    // pos: usize
    // input: &str
}

impl<I: std::fmt::Display> ParseError<I> for MyError {
    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        Self {
            message: format!("from_error_kind input={} kind={:#?}", input, kind),
        }
    }

    fn append(_input: I, _kind: ErrorKind, other: Self) -> Self {
        other
    }

    fn from_char(input: I, c: char) -> Self {
        Self {
            message: format!("from_char input={} char={}", input, c),
        }
    }
}
