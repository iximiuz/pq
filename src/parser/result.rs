use nom;

use nom_locate::LocatedSpan;

use super::error::ParseError;

pub type Span<'a> = LocatedSpan<&'a str>;

pub type IResult<'a, O> = nom::IResult<Span<'a>, O, ParseError<'a>>;

#[derive(PartialEq, Debug)]
pub enum ParseResult<T> {
    Success(T),
    Partial(&'static str),
}