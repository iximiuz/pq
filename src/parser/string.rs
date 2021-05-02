use nom::{bytes::complete::is_not, character::complete::char, sequence::delimited};

use super::error::MyError;
use super::result::IResult;

// FIXME: this is way too simplistic... Doesn't even handle escaped chars.
// Use https://github.com/Geal/nom/blob/master/examples/string.rs
pub fn string_literal(input: &str) -> IResult<&str, String> {
    let (rest, m) =
        delimited(char('"'), is_not("\""), char('"'))(input).map_err(|_: nom::Err<MyError>| {
            nom::Err::Error(MyError {
                message: "Expected string".into(),
            })
        })?;
    Ok((rest, String::from(m)))
}
