use nom;

use nom_locate::LocatedSpan;

pub type Span<'a> = LocatedSpan<&'a str>;

pub type IResult<'a, O> = nom::IResult<Span<'a>, O, ParseError<'a>>;

#[derive(PartialEq, Debug)]
pub enum ParseResult<T> {
    Complete(T),
    // (wherein, expected)
    Partial(&'static str, &'static str),
}

#[derive(Debug, PartialEq)]
pub struct ParseError<'a> {
    message: String,
    wherein: Span<'a>,
}

impl<'a> ParseError<'a> {
    pub fn new(message: String, wherein: Span<'a>) -> Self {
        Self { message, wherein }
    }

    pub fn message(&self) -> &String {
        &self.message
    }

    pub fn line(&self) -> u32 {
        self.wherein.location_line()
    }

    pub fn offset(&self) -> usize {
        self.wherein.location_offset()
    }
}

impl<'a> nom::error::ParseError<Span<'a>> for ParseError<'a> {
    fn from_error_kind(input: Span<'a>, kind: nom::error::ErrorKind) -> Self {
        Self::new(format!("parse error {:?}", kind), input)
    }

    fn append(_input: Span<'a>, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }

    fn from_char(input: Span<'a>, c: char) -> Self {
        Self::new(format!("unexpected character '{}'", c), input)
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
