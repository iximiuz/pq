use nom::{bytes::complete::is_not, character::complete::char, sequence::delimited};

use super::result::{IResult, Span};

// FIXME: this is way too simplistic... Doesn't even handle escaped chars.
// Use https://github.com/Geal/nom/blob/master/examples/string.rs
pub fn string_literal(input: Span) -> IResult<String> {
    let (rest, m) = delimited(char('"'), is_not("\""), char('"'))(input)?;
    Ok((rest, String::from(*m.fragment())))
}
