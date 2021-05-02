use nom;

use nom_locate::LocatedSpan;

use super::error::MyError;

pub type Span<'a> = LocatedSpan<&'a str>;

pub type IResult<'a, O> = nom::IResult<Span<'a>, O, MyError>;
