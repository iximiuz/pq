use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::multispace0,
    character::complete::{alpha1, alphanumeric1},
    combinator::recognize,
    multi::many0,
    sequence::pair,
    sequence::{delimited, preceded, terminated},
};

use super::result::{IResult, Span};

pub fn label_identifier(input: Span) -> IResult<String> {
    // [a-zA-Z_][a-zA-Z0-9_]*
    let (rest, m) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))(input)?;
    Ok((rest, String::from(*m.fragment())))
}

pub fn metric_identifier(input: Span) -> IResult<String> {
    // [a-zA-Z_:][a-zA-Z0-9_:]*
    let (rest, m) = recognize(pair(
        alt((alpha1, tag("_"), tag(":"))),
        many0(alt((alphanumeric1, tag("_"), tag(":")))),
    ))(input)?;
    Ok((rest, String::from(*m)))
}

pub fn maybe_padded<'a, F, O>(f: F) -> impl FnMut(Span<'a>) -> IResult<O>
where
    F: FnMut(Span<'a>) -> IResult<O>,
{
    delimited(multispace0, f, multispace0)
}

pub fn maybe_lpadded<'a, F, O>(f: F) -> impl FnMut(Span<'a>) -> IResult<O>
where
    F: FnMut(Span<'a>) -> IResult<O>,
{
    preceded(multispace0, f)
}

#[allow(dead_code)]
pub fn maybe_rpadded<'a, F, O>(f: F) -> impl FnMut(Span<'a>) -> IResult<O>
where
    F: FnMut(Span<'a>) -> IResult<O>,
{
    terminated(f, multispace0)
}
