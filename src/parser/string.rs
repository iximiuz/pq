use nom::{
    bytes::complete::is_not, character::complete::char, combinator::recognize, multi::many0,
    sequence::delimited,
};

use super::result::{IResult, Span};

// FIXME: this is way too simplistic... Doesn't even handle escaped chars.
// Use https://github.com/Geal/nom/blob/master/examples/string.rs
pub fn string_literal(input: Span) -> IResult<String> {
    let (rest, m) = recognize(delimited(char('"'), many0(is_not("\"")), char('"')))(input)?;
    let len = m.fragment().len();
    Ok((rest, m.fragment().chars().skip(1).take(len - 2).collect()))
}
