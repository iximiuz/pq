use nom;

use nom_locate::LocatedSpan;

pub type Span<'a> = LocatedSpan<&'a str>;

pub type IResult<'a, O> = nom::IResult<Span<'a>, O, ParseError<'a>>;

#[derive(Debug, PartialEq)]
pub struct ParseError<'a> {
    span: Span<'a>,
    message: Option<String>,
    wherein: Option<&'static str>,
    expected: Option<&'static str>,
}

impl<'a> ParseError<'a> {
    pub fn new(message: String, span: Span<'a>) -> Self {
        Self {
            span,
            message: Some(message),
            wherein: None,
            expected: None,
        }
    }

    pub fn partial(wherein: &'static str, expected: &'static str, span: Span<'a>) -> Self {
        Self {
            span,
            message: None,
            wherein: Some(wherein),
            expected: Some(expected),
        }
    }

    #[inline]
    pub fn span(&self) -> &Span {
        &self.span
    }

    #[inline]
    pub fn line(&self) -> u32 {
        self.span().location_line()
    }

    #[inline]
    pub fn offset(&self) -> usize {
        self.span().location_offset()
    }

    pub fn message(&self) -> String {
        if let Some(ref message) = self.message {
            return format!(
                "{}:{}: parse error: {}",
                self.line(),
                self.offset(),
                message
            );
        }

        if let (Some(wherein), Some(expected)) = (self.wherein, self.expected) {
            return format!(
                "{}:{}: parse error: unexpected '{}' in {}, expected {}",
                self.line(),
                self.offset(),
                unexpected(**self.span()),
                wherein,
                expected,
            );
        }

        unimplemented!();
    }
}

fn unexpected(found: &str) -> String {
    match found {
        "" => String::from("EOF"),
        v => format!("\"{}\"", v),
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
