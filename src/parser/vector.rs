use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, char},
    combinator::recognize,
    multi::{many0, separated_list1},
    sequence::pair,
};

use super::ast::{LabelMatcher, LabelMatchers, MatchOp, VectorSelector};
use super::common::{maybe_lpadded, maybe_padded};
use super::error::ParseError;
use super::result::{IResult, ParseResult, Span};
use super::string::string_literal;

pub fn vector_selector(input: Span) -> IResult<VectorSelector> {
    // metric_identifier label_matchers | metric_identifier | label_matchers

    let (rest, matchers) = label_matchers(input)?;
    Ok((rest, VectorSelector::new(None, matchers).unwrap())) // TODO: handle unwrap
}

// Returning type of this function may need to be converted into ParseResult.
fn label_matchers(input: Span) -> IResult<LabelMatchers> {
    // LEFT_BRACE label_match_list RIGHT_BRACE
    //   | LEFT_BRACE label_match_list COMMA RIGHT_BRACE
    //   | LEFT_BRACE RIGHT_BRACE

    let (rest, _) = char('{')(input)?;
    let (rest, _) = match maybe_lpadded(char('}'))(rest) {
        Ok((rest, _)) => return Ok((rest, LabelMatchers::empty())),
        Err(_) => (rest, '_'),
    };

    let (rest, matchers) = match maybe_lpadded(label_match_list)(rest) {
        Ok((r, ParseResult::Success(m))) => (r, m),
        Ok((r, ParseResult::Partial(diag))) => {
            return Err(nom::Err::Error(ParseError::new(
                *r,
                (r.location_line(), r.location_offset()),
                "label matching",
                diag,
            )));
        }
        Err(_) => {
            return Err(nom::Err::Error(ParseError::new(
                *rest,
                (rest.location_line(), rest.location_offset()),
                "label matching",
                r#"identifier or "}""#,
            )))
        }
    };

    // Handling trailing comma.
    let (rest, _) = match maybe_lpadded(char(','))(rest) {
        Ok((rest, c)) => (rest, c),
        Err(_) => (rest, '_'),
    };

    let (rest, _) = maybe_lpadded(char('}'))(rest).map_err(|_: nom::Err<ParseError>| {
        nom::Err::Error(ParseError::new(
            *rest,
            (rest.location_line(), rest.location_offset()),
            "label matching",
            r#"identifier or "}""#,
        ))
    })?;

    Ok((rest, LabelMatchers::new(matchers)))
}

/// Parses a non-empty list of label matches separated by a comma.
/// No trailing commas allowed.
fn label_match_list(input: Span) -> IResult<ParseResult<Vec<LabelMatcher>>> {
    // label_match_list COMMA label_matcher | label_matcher

    let (rest, matches) = separated_list1(tag(","), maybe_padded(label_matcher))(input)?;
    let mut matchers = vec![];

    for m in matches.into_iter() {
        match m {
            ParseResult::Success(m) => matchers.push(m),
            ParseResult::Partial(diag) => return Ok((rest, ParseResult::Partial(diag))),
        };
    }

    Ok((rest, ParseResult::Success(matchers)))
}

/// label_matcher actually never returns IResult::Err.
/// Instead of an error, a partial ParseResult::Partial is returned.
fn label_matcher(input: Span) -> IResult<ParseResult<LabelMatcher>> {
    // IDENTIFIER match_op STRING

    let (rest, label) = label_identifier(input)?;

    let (rest, op) = match maybe_lpadded(match_op)(rest) {
        Ok(v) => v,
        Err(_) => {
            return Ok((
                rest,
                ParseResult::Partial(r#"one of "=", "!=", "=~", "!~""#),
            ));
        }
    };

    let (rest, value) = match maybe_lpadded(string_literal)(rest) {
        Ok(v) => v,
        Err(_) => {
            return Ok((rest, ParseResult::Partial("label value as string literal")));
        }
    };

    Ok((
        rest,
        ParseResult::Success(LabelMatcher::new(label, op, value)),
    ))
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

    #[test]
    fn test_label_matchers_valid() -> std::result::Result<(), String> {
        let tests = [
            ("{}", vec![]),
            ("{ }", vec![]),
            ("{   }", vec![]),
            ("{   }  ", vec![]),
            ("{} or", vec![]),
            (
                r#"{foo!~"123 qux"}"#,
                vec![("foo", MatchOp::NeqRe, "123 qux")],
            ),
            (
                r#"{foo!~"123 qux",bar="42"}"#,
                vec![
                    ("foo", MatchOp::NeqRe, "123 qux"),
                    ("bar", MatchOp::Eql, "42"),
                ],
            ),
            (r#"{ foo="bar"}"#, vec![("foo", MatchOp::Eql, "bar")]),
            (r#"{  foo="bar"}"#, vec![("foo", MatchOp::Eql, "bar")]),
            (r#"{foo="bar",}"#, vec![("foo", MatchOp::Eql, "bar")]),
            (r#"{foo="bar" ,}"#, vec![("foo", MatchOp::Eql, "bar")]),
            (r#"{foo="bar"  ,}"#, vec![("foo", MatchOp::Eql, "bar")]),
            (r#"{foo="bar"  , }"#, vec![("foo", MatchOp::Eql, "bar")]),
            (r#"{foo="bar"  ,  }"#, vec![("foo", MatchOp::Eql, "bar")]),
            (
                r#"{foo="bar",qux="123"}"#,
                vec![("foo", MatchOp::Eql, "bar"), ("qux", MatchOp::Eql, "123")],
            ),
            (
                r#"{foo="bar", qux="123"}"#,
                vec![("foo", MatchOp::Eql, "bar"), ("qux", MatchOp::Eql, "123")],
            ),
            (
                r#"{foo="bar" , qux="123"}"#,
                vec![("foo", MatchOp::Eql, "bar"), ("qux", MatchOp::Eql, "123")],
            ),
            (
                r#"{foo="bar", qux="123", abc="xyz"}"#,
                vec![
                    ("foo", MatchOp::Eql, "bar"),
                    ("qux", MatchOp::Eql, "123"),
                    ("abc", MatchOp::Eql, "xyz"),
                ],
            ),
            (
                r#"{foo="bar", qux="123" , abc="xyz"}"#,
                vec![
                    ("foo", MatchOp::Eql, "bar"),
                    ("qux", MatchOp::Eql, "123"),
                    ("abc", MatchOp::Eql, "xyz"),
                ],
            ),
            (r#"{ foo ="bar"}"#, vec![("foo", MatchOp::Eql, "bar")]),
            (r#"{ foo= "bar"}"#, vec![("foo", MatchOp::Eql, "bar")]),
            (r#"{ foo = "bar"}"#, vec![("foo", MatchOp::Eql, "bar")]),
            (
                r#"{    foo   =    "bar",   qux    =   "123"    }"#,
                vec![("foo", MatchOp::Eql, "bar"), ("qux", MatchOp::Eql, "123")],
            ),
        ];

        for (input, expected_matchers) in tests.iter() {
            let (_, actual_matchers) =
                label_matchers(Span::new(&input)).map_err(|e| ParseError::from(e))?;
            assert_eq!(
                actual_matchers,
                LabelMatchers::new(
                    expected_matchers
                        .iter()
                        .map(|(l, o, v)| LabelMatcher::new(*l, *o, *v))
                        .collect()
                )
            );
        }
        Ok(())
    }

    #[test]
    fn test_label_matchers_invalid() {
        let tests = [
            ("{", "", (1, 1), "label matching", r#"identifier or "}""#),
            (
                "{123",
                "123",
                (1, 1),
                "label matching",
                r#"identifier or "}""#,
            ),
            (
                "{,}",
                ",}",
                (1, 1),
                "label matching",
                r#"identifier or "}""#,
            ),
            (
                r#"{foo!~"123 qux",,}"#,
                r#",}"#,
                (1, 16),
                "label matching",
                r#"identifier or "}""#,
            ),
            (
                "{foo",
                "",
                (1, 4),
                "label matching",
                r#"one of "=", "!=", "=~", "!~""#,
            ),
            (
                r#"{foo="bar",f12=}"#,
                r#"}"#,
                (1, 15),
                "label matching",
                "label value as string literal",
            ),
            (
                r#"{foo="bar",baz=~"42",qux!}"#,
                r#"!}"#,
                (1, 24),
                "label matching",
                r#"one of "=", "!=", "=~", "!~""#,
            ),
        ];

        for &(input, unexpected, error_pos, where_in, expected) in tests.iter() {
            assert_eq!(
                label_matchers(Span::new(input)),
                Err(nom::Err::Error(ParseError::new(
                    unexpected, error_pos, where_in, expected,
                ))),
                "while testing input '{}'",
                input,
            );
        }
    }

    #[test]
    fn test_label_match_list_valid() -> std::result::Result<(), String> {
        let tests = [
            (
                r#"foo!~"123 qux""#,
                vec![("foo", MatchOp::NeqRe, "123 qux")],
            ),
            (
                r#"foo!~"123 qux","#,
                vec![("foo", MatchOp::NeqRe, "123 qux")],
            ),
            (
                r#"foo!~"123 qux",bar="42""#,
                vec![
                    ("foo", MatchOp::NeqRe, "123 qux"),
                    ("bar", MatchOp::Eql, "42"),
                ],
            ),
        ];

        for (input, expected_matchers) in tests.iter() {
            let actual_matchers =
                match label_match_list(Span::new(&input)).map_err(|e| ParseError::from(e))? {
                    (_, ParseResult::Success(m)) => m,
                    (_, ParseResult::Partial(_)) => panic!("oops"),
                };

            assert_eq!(actual_matchers.len(), expected_matchers.len());
            for (matcher, (label, match_op, value)) in
                actual_matchers.iter().zip(expected_matchers.iter())
            {
                assert_eq!(*matcher, LabelMatcher::new(*label, *match_op, *value),);
            }
        }
        Ok(())
    }

    #[test]
    fn test_label_match_list_invalid() {
        assert!(label_match_list(Span::new("")).is_err());
        assert!(label_match_list(Span::new(",")).is_err());
        assert!(label_match_list(Span::new(",,")).is_err());
        assert!(label_match_list(Span::new(", ,")).is_err());
    }

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
                ParseResult::Success(LabelMatcher::new(label, op, value))
            );
        }
        Ok(())
    }

    #[test]
    fn test_label_matcher_partial() {
        let tests: &[(&str, &str, &str, (u32, usize))] = &[
            ("foo!", "!", r#"one of "=", "!=", "=~", "!~""#, (1, 3)),
            ("foo!=", "", r#"label value as string literal"#, (1, 5)),
            ("foo!= ", " ", r#"label value as string literal"#, (1, 5)),
            ("foo!=,", ",", r#"label value as string literal"#, (1, 5)),
            (
                "foo!=123",
                "123",
                r#"label value as string literal"#,
                (1, 5),
            ),
        ];

        for &(input, output, error_msg, error_pos) in tests.iter() {
            match label_matcher(Span::new(input)) {
                Ok((span, ParseResult::Partial(diag))) => {
                    assert_eq!(*span, output);
                    assert_eq!((span.location_line(), span.location_offset()), error_pos);
                    assert_eq!(diag, error_msg);
                }
                Ok(res) => panic!("ParseResult::Partial expected but found {:#?}", res),
                Err(err) => panic!("ParseResult::Partial expected but found {:#?}", err),
            };
        }
    }

    #[test]
    fn test_label_matcher_invalid() {
        // We don't care about actual error, just the fact that it errored.
        let tests = ["", ",", "123", "1foo="];

        for input in tests.iter() {
            let res = label_matcher(Span::new(input));
            assert!(res.is_err(), "Error expected but found {:#?}", res);
        }
    }

    #[test]
    fn test_label_identifier_valid() -> std::result::Result<(), String> {
        let tests = [
            ("l", "l"),
            ("l123", "l123"),
            ("_", "_"),
            ("_1", "_1"),
            ("label", "label"),
            ("_label", "_label"),
            ("label_123", "label_123"),
            ("label_123_", "label_123_"),
            ("label_123_ ", "label_123_"),
            ("label_123_{}", "label_123_"),
            ("label_123_ {}", "label_123_"),
            ("label_123_ {}", "label_123_"),
            ("label_123_ or", "label_123_"),
            ("label_123_-", "label_123_"),
        ];

        for &(input, expected_label) in tests.iter() {
            let (_, actual_label) =
                label_identifier(Span::new(input)).map_err(|e| ParseError::from(e))?;
            assert_eq!(expected_label, actual_label);
        }

        Ok(())
    }

    #[test]
    fn test_label_identifier_invalid() {
        assert!(label_identifier(Span::new("1")).is_err());
        assert!(label_identifier(Span::new("1_")).is_err());
        assert!(label_identifier(Span::new("123label")).is_err());
    }

    #[test]
    fn test_match_op_valid() -> std::result::Result<(), String> {
        let tests = [
            (r#"="foo""#, r#""foo""#, MatchOp::Eql),
            (r#"!="foo""#, r#""foo""#, MatchOp::Neq),
            (r#"=~"foo""#, r#""foo""#, MatchOp::EqlRe),
            (r#"!~"foo""#, r#""foo""#, MatchOp::NeqRe),
        ];

        for &(input, expected_remainder, expected_match_op) in tests.iter() {
            let (actual_remainder, actual_match_op) =
                match_op(Span::new(input)).map_err(|e| ParseError::from(e))?;
            assert_eq!(expected_match_op, actual_match_op);
            assert_eq!(expected_remainder, *actual_remainder);
        }
        Ok(())
    }

    #[test]
    fn test_match_op_invalid() {
        assert!(match_op(Span::new("a\"foo\"")).is_err());
        assert!(match_op(Span::new("!\"foo\"")).is_err());
        assert!(match_op(Span::new("~\"foo\"")).is_err());
    }
}
