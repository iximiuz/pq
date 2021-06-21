use std::convert::TryFrom;

use nom::{
    branch::alt, bytes::complete::tag_no_case, character::complete::char, sequence::terminated,
};

use super::ast::{Expr, FunctionCall};
use super::common::maybe_lpadded;
use super::result::{IResult, ParseError, Span};
use super::vector::vector_selector;
use crate::error::{Error, Result};

pub(super) fn expr_function_call(input: Span) -> IResult<Expr> {
    let (rest, func_id) = terminated(function_identifier, maybe_lpadded(char('(')))(input)?;

    // function_call should never return nom::Err::Error
    let (rest, func_call) = function_call(func_id, rest)?;

    let (rest, _) = match maybe_lpadded(char(')'))(rest) {
        Ok((rest, c)) => (rest, c),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "function call",
                ")",
                rest,
            )))
        }
        Err(e) => return Err(e),
    };

    Ok((rest, Expr::FunctionCall(func_call)))
}

enum FunctionIdentifier {
    CountOverTime,
}

impl std::convert::TryFrom<&str> for FunctionIdentifier {
    type Error = Error;

    fn try_from(id: &str) -> Result<Self> {
        use FunctionIdentifier::*;

        match id.to_lowercase().as_str() {
            "count_over_time" => Ok(CountOverTime),
            _ => Err(Error::new("Unknown function identifier")),
        }
    }
}

fn function_identifier(input: Span) -> IResult<FunctionIdentifier> {
    let (rest, id) = alt((
        tag_no_case("count_over_time"),
        tag_no_case("count_over_time"),
    ))(input)?;
    Ok((rest, FunctionIdentifier::try_from(*id).unwrap()))
}

/// It should never return nom::Err::Error. Only success or total failure.
fn function_call(func_id: FunctionIdentifier, input: Span) -> IResult<FunctionCall> {
    let parse_result = match func_id {
        FunctionIdentifier::CountOverTime => func_count_over_time(input),
    };

    match parse_result {
        Ok((rest, func_call)) => Ok((rest, func_call)),
        Err(nom::Err::Error(_)) => panic!("bug!"),
        Err(e) => Err(e),
    }
}

fn func_count_over_time(input: Span) -> IResult<FunctionCall> {
    match vector_selector(input) {
        Ok((rest, selector)) if selector.duration().is_some() => {
            Ok((rest, FunctionCall::CountOverTime(selector)))
        }
        Err(nom::Err::Failure(e)) => Err(nom::Err::Failure(e)),
        _ => Err(nom::Err::Failure(ParseError::partial(
            "count_over_time()",
            "range vector selector",
            input,
        ))),
    }
}
