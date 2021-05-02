use nom;

use nom_locate::LocatedSpan;

use super::error::ParseError;

pub type Span<'a> = LocatedSpan<&'a str>;

pub type IResult<'a, O> = nom::IResult<Span<'a>, O, ParseError>;

pub fn unexpected(found: &str) -> String {
    match found {
        "" => String::from("EOF"),
        v => format!("\"{}\"", v),
    }
}
