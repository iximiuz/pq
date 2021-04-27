#![allow(warnings)]

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{alpha1, alphanumeric1, char},
    combinator::recognize,
    multi::{many0, separated_list0},
    sequence::{delimited, pair, tuple},
    IResult,
};

use crate::error::{Error, Result};

#[derive(Debug)]
pub struct AST {
    root: NodeKind,
}

#[derive(Debug)]
enum NodeKind {
    VectorSelector(i32),
}

pub fn parse(input: &str) -> Result<AST> {
    Ok(AST {
        root: NodeKind::VectorSelector(42),
    })
}

struct VectorSelector {}

fn vector_selector(input: &str) -> IResult<&str, VectorSelector> {
    // metric_identifier label_matchers | metric_identifier | label_matchers
    Ok((input, VectorSelector {}))
}

fn label_matchers() {
    // LEFT_BRACE label_match_list RIGHT_BRACE | LEFT_BRACE label_match_list COMMA RIGHT_BRACE
    //   | LEFT_BRACE RIGHT_BRACE
}

fn label_match_list(input: &str) -> IResult<&str, Vec<LabelMatcher>> {
    // label_match_list COMMA label_matcher | label_matcher
    let (rest, matchers) = separated_list0(tag(","), label_matcher)(input)?;
    Ok((rest, matchers))
}

#[derive(Debug, Eq, PartialEq)]
struct LabelMatcher {
    label: String,
    match_op: MatchOp,
    value: String,
}

fn label_matcher(input: &str) -> IResult<&str, LabelMatcher> {
    // IDENTIFIER match_op STRING
    let (rest, (label, match_op, value)) =
        tuple((label_identifier, match_op, string_literal))(input)?;
    Ok((
        rest,
        LabelMatcher {
            label,
            match_op,
            value,
        },
    ))
}

// FIXME: this is way too simplistic... Doesn't even handle escaped chars.
// Use https://github.com/Geal/nom/blob/master/examples/string.rs
fn string_literal(input: &str) -> IResult<&str, String> {
    let (rest, m) = delimited(char('"'), is_not("\""), char('"'))(input)?;
    Ok((rest, String::from(m)))
}

fn label_identifier(input: &str) -> IResult<&str, String> {
    // [a-zA-Z_][a-zA-Z0-9_]*
    let (rest, m) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))(input)?;
    Ok((rest, String::from(m)))
}

fn metric_identifier(input: &str) -> IResult<&str, String> {
    // [a-zA-Z_:][a-zA-Z0-9_:]*
    let (rest, m) = recognize(pair(
        alt((alpha1, tag("_"), tag(":"))),
        many0(alt((alphanumeric1, tag("_"), tag(":")))),
    ))(input)?;
    Ok((rest, String::from(m)))
}

#[derive(Debug, Eq, PartialEq)]
enum MatchOp {
    eql,
    neq,
    eql_re,
    neq_re,
}

fn match_op(input: &str) -> IResult<&str, MatchOp> {
    let (rest, m) = alt((tag("=~"), tag("!~"), tag("!="), tag("=")))(input)?;
    match m {
        "=" => Ok((rest, MatchOp::eql)),
        "!=" => Ok((rest, MatchOp::neq)),
        "=~" => Ok((rest, MatchOp::eql_re)),
        "!~" => Ok((rest, MatchOp::neq_re)),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_identifier() {
        // valid labels
        assert_eq!(label_identifier("l"), Ok(("", String::from("l"))));
        assert_eq!(label_identifier("l123"), Ok(("", String::from("l123"))));
        assert_eq!(label_identifier("_"), Ok(("", String::from("_"))));
        assert_eq!(label_identifier("_1"), Ok(("", String::from("_1"))));
        assert_eq!(label_identifier("label"), Ok(("", String::from("label"))));
        assert_eq!(label_identifier("_label"), Ok(("", String::from("_label"))));
        assert_eq!(
            label_identifier("label_123"),
            Ok(("", String::from("label_123")))
        );
        assert_eq!(
            label_identifier("label_123_"),
            Ok(("", String::from("label_123_")))
        );
        assert_eq!(
            label_identifier("label_123_{}"),
            Ok(("{}", String::from("label_123_")))
        );

        // invalid labels
        assert!(label_identifier("1").is_err());
        assert!(label_identifier("1_").is_err());
        assert!(label_identifier("123label").is_err());
    }

    #[test]
    fn test_match_op() {
        assert_eq!(match_op("=\"foo\""), Ok(("\"foo\"", MatchOp::eql)));
        assert_eq!(match_op("!=\"foo\""), Ok(("\"foo\"", MatchOp::neq)));
        assert_eq!(match_op("=~\"foo\""), Ok(("\"foo\"", MatchOp::eql_re)));
        assert_eq!(match_op("!~\"foo\""), Ok(("\"foo\"", MatchOp::neq_re)));
    }

    #[test]
    fn test_label_matcher() {
        assert_eq!(
            label_matcher("foo=\"bar\""),
            Ok((
                "",
                LabelMatcher {
                    label: String::from("foo"),
                    match_op: MatchOp::eql,
                    value: String::from("bar")
                }
            ))
        );

        assert_eq!(
            label_matcher("foo!~\"123 qux\""),
            Ok((
                "",
                LabelMatcher {
                    label: String::from("foo"),
                    match_op: MatchOp::neq_re,
                    value: String::from("123 qux")
                }
            ))
        );
    }

    #[test]
    fn test_label_match_list() {
        assert_eq!(
            label_match_list("foo!~\"123 qux\""),
            Ok((
                "",
                vec![LabelMatcher {
                    label: String::from("foo"),
                    match_op: MatchOp::neq_re,
                    value: String::from("123 qux")
                }]
            ))
        );

        assert_eq!(
            label_match_list("foo!~\"123 qux\","),
            Ok((
                ",",
                vec![LabelMatcher {
                    label: String::from("foo"),
                    match_op: MatchOp::neq_re,
                    value: String::from("123 qux")
                }]
            ))
        );

        assert_eq!(
            label_match_list("foo!~\"123 qux\",bar=\"42\""),
            Ok((
                "",
                vec![
                    LabelMatcher {
                        label: String::from("foo"),
                        match_op: MatchOp::neq_re,
                        value: String::from("123 qux")
                    },
                    LabelMatcher {
                        label: String::from("bar"),
                        match_op: MatchOp::eql,
                        value: String::from("42")
                    },
                ]
            ))
        );
    }
}
