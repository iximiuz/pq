use nom;

use crate::error;

#[derive(Debug, PartialEq)]
pub struct ParseError<'a> {
    unexpected: &'a str,
    position: (u32, usize),
    where_in: &'static str,
    expected: &'static str,
}

impl<'a> ParseError<'a> {
    pub fn new(
        unexpected: &'a str,
        position: (u32, usize),
        where_in: &'static str,
        expected: &'static str,
    ) -> Self {
        Self {
            unexpected,
            position,
            where_in,
            expected,
        }
    }

    pub fn line(&self) -> u32 {
        self.position.0
    }

    pub fn offset(&self) -> usize {
        self.position.1
    }
}

impl<'a, I: std::fmt::Display> nom::error::ParseError<I> for ParseError<'a> {
    fn from_error_kind(_input: I, _kind: nom::error::ErrorKind) -> Self {
        Self::new("from_error_kind", (0, 0), "", "")
    }

    fn append(_input: I, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }

    fn from_char(_input: I, _c: char) -> Self {
        Self::new("from_char", (0, 0), "", "")
    }
}

impl<'a> From<ParseError<'a>> for String {
    fn from(err: ParseError) -> Self {
        format!(
            "{}:{} unexpected {} in {}, expected {}",
            err.line(),
            err.offset(),
            unexpected(err.unexpected),
            err.where_in,
            err.expected,
        )
    }
}

impl<'a> From<ParseError<'a>> for error::Error {
    fn from(err: ParseError) -> Self {
        error::Error::new(&String::from(err))
    }
}

fn unexpected(found: &str) -> String {
    match found {
        "" => String::from("EOF"),
        v => format!("\"{}\"", v),
    }
}

impl<'a> From<nom::Err<ParseError<'a>>> for ParseError<'a> {
    fn from(err: nom::Err<ParseError<'a>>) -> Self {
        match err {
            nom::Err::Error(e) | nom::Err::Failure(e) => e,
            nom::Err::Incomplete(_) => unreachable!(),
        }
    }
}
