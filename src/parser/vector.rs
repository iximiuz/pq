use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1},
    combinator::recognize,
    multi::{many0, many1, separated_list1},
    sequence::{delimited, pair},
};

use super::ast::{LabelMatcher, LabelMatchers, MatchOp, VectorSelector};
use super::error::ParseError;
use super::result::{unexpected, IResult, Span};
use super::string::string_literal;

pub fn vector_selector(input: Span) -> IResult<VectorSelector> {
    // metric_identifier label_matchers | metric_identifier | label_matchers
    let (rest, matchers) = label_matchers(input)?;
    Ok((rest, VectorSelector::new(None, matchers).unwrap())) // TODO: handle unwrap
}

fn label_matchers(input: Span) -> IResult<LabelMatchers> {
    // LEFT_BRACE label_match_list RIGHT_BRACE
    //   | LEFT_BRACE label_match_list COMMA RIGHT_BRACE
    //   | LEFT_BRACE RIGHT_BRACE
    let (rest, m) = alt((
        delimited(tag("{"), many0(label_match_list), tag("}")),
        delimited(tag("{"), many1(label_match_list), tag(",}")),
    ))(input)?;
    Ok((
        rest,
        LabelMatchers::new(m.into_iter().flatten().collect::<Vec<_>>()),
    ))
}

fn label_match_list(input: Span) -> IResult<Vec<LabelMatcher>> {
    // label_match_list COMMA label_matcher | label_matcher
    separated_list1(tag(","), label_matcher)(input)
}

fn label_matcher(input: Span) -> IResult<LabelMatcher> {
    // IDENTIFIER match_op STRING

    // Original version were producing very poor diagnostic messages:
    // let (rest, (label, match_op, value)) =
    //    tuple((label_identifier, match_op, string_literal))(input)?;

    let (rest, label) = label_identifier(input).map_err(|_| {
        nom::Err::Error(ParseError::new(
            format!(
                "unexpected {} in label matching, expected identifier or \"}}\"",
                unexpected(*input),
            ),
            input.location_offset(),
            input.location_line(),
        ))
    })?;

    let (rest, op) = match_op(rest).map_err(|_| {
        nom::Err::Error(ParseError::new(
            format!(
                "unexpected {} in label matching, expected one of \"=\", \"!=\", \"=~\", \"!~\"",
                unexpected(*rest),
            ),
            rest.location_offset(),
            rest.location_line(),
        ))
    })?;

    let (rest, value) = string_literal(rest).map_err(|_| {
        nom::Err::Error(ParseError::new(
            format!(
                "unexpected {} in label matching, expected string label value",
                unexpected(*rest),
            ),
            rest.location_offset(),
            rest.location_line(),
        ))
    })?;

    Ok((rest, LabelMatcher::new(label, op, value)))
}

fn label_identifier(input: Span) -> IResult<String> {
    // [a-zA-Z_][a-zA-Z0-9_]*
    let (rest, m) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))(input)?;
    Ok((rest, String::from(*m.fragment())))
}

fn match_op(input: Span) -> IResult<MatchOp> {
    let (rest, m) = alt((tag("=~"), tag("!~"), tag("!="), tag("=")))(input)?;
    match *m.fragment() {
        "=" => Ok((rest, MatchOp::Eql)),
        "!=" => Ok((rest, MatchOp::Neq)),
        "=~" => Ok((rest, MatchOp::EqlRe)),
        "!~" => Ok((rest, MatchOp::NeqRe)),
        _ => unreachable!(),
    }
}

// fn metric_identifier(input: Span) -> IResult<String> {
//     // [a-zA-Z_:][a-zA-Z0-9_:]*
//     let (rest, m) = recognize(pair(
//         alt((alpha1, tag("_"), tag(":"))),
//         many0(alt((alphanumeric1, tag("_"), tag(":")))),
//     ))(input)?;
//     Ok((rest, String::from(m)))
// }

#[cfg(test)]
mod tests {
    use super::*;

    //     #[test]
    //     fn test_label_matchers_valid() {
    //         assert_eq!(label_matchers("{}"), Ok(("", LabelMatchers::new(vec![]))));
    //         assert_eq!(
    //             label_matchers("{} or"),
    //             Ok((" or", LabelMatchers::new(vec![])))
    //         );
    //
    //         assert_eq!(
    //             label_matchers("{foo!~\"123 qux\"}"),
    //             Ok((
    //                 "",
    //                 LabelMatchers::new(vec![LabelMatcher::new("foo", MatchOp::NeqRe, "123 qux")])
    //             ))
    //         );
    //
    //         assert_eq!(
    //             label_matchers("{foo!~\"123 qux\",}"),
    //             Ok((
    //                 "",
    //                 LabelMatchers::new(vec![LabelMatcher::new("foo", MatchOp::NeqRe, "123 qux")])
    //             ))
    //         );
    //
    //         assert_eq!(
    //             label_matchers("{foo!~\"123 qux\",bar=\"42\"}"),
    //             Ok((
    //                 "",
    //                 LabelMatchers::new(vec![
    //                     LabelMatcher::new("foo", MatchOp::NeqRe, "123 qux"),
    //                     LabelMatcher::new("bar", MatchOp::Eql, "42"),
    //                 ])
    //             ))
    //         );
    //     }
    //
    //     #[test]
    //     fn test_label_matchers_invalid() {
    //         assert!(label_matchers("{,}").is_err());
    //         assert!(label_matchers("{foo!~\"123 qux\",,}").is_err());
    //     }
    //
    //     #[test]
    //     fn test_label_match_list_valid() {
    //         assert_eq!(
    //             label_match_list("foo!~\"123 qux\""),
    //             Ok((
    //                 "",
    //                 vec![LabelMatcher::new("foo", MatchOp::NeqRe, "123 qux")]
    //             ))
    //         );
    //
    //         assert_eq!(
    //             label_match_list("foo!~\"123 qux\","),
    //             Ok((
    //                 ",",
    //                 vec![LabelMatcher::new("foo", MatchOp::NeqRe, "123 qux")]
    //             ))
    //         );
    //
    //         assert_eq!(
    //             label_match_list("foo!~\"123 qux\",bar=\"42\""),
    //             Ok((
    //                 "",
    //                 vec![
    //                     LabelMatcher::new("foo", MatchOp::NeqRe, "123 qux"),
    //                     LabelMatcher::new("bar", MatchOp::Eql, "42"),
    //                 ]
    //             ))
    //         );
    //     }
    //
    //     #[test]
    //     fn test_label_match_list_invalid() {
    //         assert!(label_match_list("").is_err());
    //         assert!(label_match_list(",").is_err());
    //         assert!(label_match_list(",,").is_err());
    //         assert!(label_match_list(", ,").is_err());
    //     }

    #[test]
    fn test_label_matcher_valid() -> std::result::Result<(), String> {
        let tests = [
            (r#"foo="bar""#, ("foo", MatchOp::Eql, "bar")),
            (r#"foo!~"123 qux""#, ("foo", MatchOp::NeqRe, "123 qux")),
        ];

        for &(input, (label, op, value)) in tests.iter() {
            assert_eq!(
                label_matcher(Span::new(input))
                    .map_err(|e| ParseError::from(e))?
                    .1,
                LabelMatcher::new(label, op, value)
            );
        }
        Ok(())
    }

    #[test]
    fn test_label_matcher_invalid() {
        let tests: &[(&str, &str, (usize, u32))] = &[
            (
                "",
                r#"unexpected EOF in label matching, expected identifier or "}""#,
                (0, 1),
            ),
            (
                "123",
                r#"unexpected "123" in label matching, expected identifier or "}""#,
                (0, 1),
            ),
            (
                "foo",
                r#"unexpected EOF in label matching, expected one of "=", "!=", "=~", "!~""#,
                (3, 1),
            ),
            (
                "foo!",
                r#"unexpected "!" in label matching, expected one of "=", "!=", "=~", "!~""#,
                (3, 1),
            ),
            (
                "foo!=",
                r#"unexpected EOF in label matching, expected string label value"#,
                (5, 1),
            ),
            (
                "foo!=123",
                r#"unexpected "123" in label matching, expected string label value"#,
                (5, 1),
            ),
        ];

        for &(input, error, pos) in tests.iter() {
            assert_eq!(
                label_matcher(Span::new(input)),
                Err(nom::Err::Error(ParseError::new(
                    String::from(error),
                    pos.0,
                    pos.1,
                )))
            );
        }
    }

    //     #[test]
    //     fn test_label_identifier_valid() {
    //         assert_eq!(label_identifier("l"), Ok(("", String::from("l"))));
    //         assert_eq!(label_identifier("l123"), Ok(("", String::from("l123"))));
    //         assert_eq!(label_identifier("_"), Ok(("", String::from("_"))));
    //         assert_eq!(label_identifier("_1"), Ok(("", String::from("_1"))));
    //         assert_eq!(label_identifier("label"), Ok(("", String::from("label"))));
    //         assert_eq!(label_identifier("_label"), Ok(("", String::from("_label"))));
    //         assert_eq!(
    //             label_identifier("label_123"),
    //             Ok(("", String::from("label_123")))
    //         );
    //         assert_eq!(
    //             label_identifier("label_123_"),
    //             Ok(("", String::from("label_123_")))
    //         );
    //         assert_eq!(
    //             label_identifier("label_123_{}"),
    //             Ok(("{}", String::from("label_123_")))
    //         );
    //     }
    //
    //     #[test]
    //     fn test_label_identifier_invalid() {
    //         assert!(label_identifier("1").is_err());
    //         assert!(label_identifier("1_").is_err());
    //         assert!(label_identifier("123label").is_err());
    //     }

    // #[test]
    // fn test_match_op_valid() {
    //     assert_eq!(
    //         match_op(Span::new("=\"foo\"")),
    //         Ok(("\"foo\"", MatchOp::Eql))
    //     );
    //     assert_eq!(
    //         match_op(Span::new("!=\"foo\"")),
    //         Ok(("\"foo\"", MatchOp::Neq))
    //     );
    //     assert_eq!(
    //         match_op(Span::new("=~\"foo\"")),
    //         Ok(("\"foo\"", MatchOp::EqlRe))
    //     );
    //     assert_eq!(
    //         match_op(Span::new("!~\"foo\"")),
    //         Ok(("\"foo\"", MatchOp::NeqRe))
    //     );
    // }

    #[test]
    fn test_match_op_invalid() {
        assert!(match_op(Span::new("a\"foo\"")).is_err());
        assert!(match_op(Span::new("!\"foo\"")).is_err());
        assert!(match_op(Span::new("~\"foo\"")).is_err());
    }
}
