use nom::{
    character::complete::multispace0,
    sequence::{delimited, preceded, terminated},
};

use super::result::{IResult, Span};

pub fn maybe_padded<'a, F, O>(f: F) -> impl FnMut(Span<'a>) -> IResult<O>
where
    F: Fn(Span<'a>) -> IResult<O>,
{
    delimited(multispace0, f, multispace0)
}

pub fn maybe_lpadded<'a, F, O>(f: F) -> impl FnMut(Span<'a>) -> IResult<O>
where
    F: Fn(Span<'a>) -> IResult<O>,
{
    preceded(multispace0, f)
}

pub fn maybe_rpadded<'a, F, O>(f: F) -> impl FnMut(Span<'a>) -> IResult<O>
where
    F: Fn(Span<'a>) -> IResult<O>,
{
    terminated(f, multispace0)
}
