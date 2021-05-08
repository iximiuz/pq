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

pub fn vector_selector(input: Span) -> IResult<ParseResult<VectorSelector>> {
    //   metric_identifier label_matchers
    // | metric_identifier
    // | label_matchers

    let (rest, metric) = match metric_identifier(input) {
        Ok((r, m)) => (r, Some(m)),
        Err(_) => (input, None),
    };

    let (rest, matchers) = match maybe_lpadded(label_matchers)(rest) {
        Ok((r, ParseResult::Partial(w, e))) => return Ok((r, ParseResult::Partial(w, e))),
        Ok((r, ParseResult::Complete(m))) => (r, m),
        Err(_) if metric.is_some() => (rest, LabelMatchers::empty()),
        Err(e) => return Err(nom::Err::Error(ParseError::from(e))),
    };

    Ok((
        rest,
        ParseResult::Complete(VectorSelector::new(metric, matchers).unwrap()),
    ))
}

fn label_matchers(input: Span) -> IResult<ParseResult<LabelMatchers>> {
    //   LEFT_BRACE label_match_list RIGHT_BRACE
    // | LEFT_BRACE label_match_list COMMA RIGHT_BRACE
    // | LEFT_BRACE RIGHT_BRACE

    let (rest, _) = char('{')(input)?;

    let (rest, matchers) = match maybe_lpadded(label_match_list)(rest) {
        Ok((r, ParseResult::Partial(w, e))) => return Ok((r, ParseResult::Partial(w, e))),
        Ok((r, ParseResult::Complete(m))) => (r, m),
        Err(_) => (rest, vec![]),
    };

    // Chop off a possible trailing comma, but only matchers list is not empty.
    let (rest, _) = match matchers.len() {
        0 => (rest, '_'),
        _ => maybe_lpadded(char(','))(rest).unwrap_or((rest, '_')),
    };

    Ok(match maybe_lpadded(char('}'))(rest) {
        Ok((r, _)) => (r, ParseResult::Complete(LabelMatchers::new(matchers))),
        Err(_) => (
            rest,
            ParseResult::Partial("label matching", r#"identifier or "}""#),
        ),
    })
}

/// Parses a non-empty list of label matches separated by a comma.
/// No trailing commas allowed.
fn label_match_list(input: Span) -> IResult<ParseResult<Vec<LabelMatcher>>> {
    //   label_match_list COMMA label_matcher
    // | label_matcher

    let (rest, matches) = separated_list1(tag(","), maybe_padded(label_matcher))(input)?;
    let mut matchers = vec![];

    for m in matches.into_iter() {
        match m {
            ParseResult::Complete(m) => matchers.push(m),
            ParseResult::Partial(w, e) => return Ok((rest, ParseResult::Partial(w, e))),
        };
    }

    Ok((rest, ParseResult::Complete(matchers)))
}

fn label_matcher(input: Span) -> IResult<ParseResult<LabelMatcher>> {
    // IDENTIFIER match_op STRING

    let (rest, label) = label_identifier(input)?;

    let (rest, op) = match maybe_lpadded(match_op)(rest) {
        Ok(v) => v,
        Err(_) => {
            return Ok((
                rest,
                ParseResult::Partial("label matching", r#"one of "=", "!=", "=~", "!~""#),
            ));
        }
    };

    let (rest, value) = match maybe_lpadded(string_literal)(rest) {
        Ok(v) => v,
        Err(_) => {
            return Ok((
                rest,
                ParseResult::Partial("label matching", "label value as string literal"),
            ));
        }
    };

    Ok((
        rest,
        ParseResult::Complete(LabelMatcher::new(label, op, value)),
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

fn metric_identifier(input: Span) -> IResult<String> {
    // [a-zA-Z_:][a-zA-Z0-9_:]*
    let (rest, m) = recognize(pair(
        alt((alpha1, tag("_"), tag(":"))),
        many0(alt((alphanumeric1, tag("_"), tag(":")))),
    ))(input)?;
    Ok((rest, String::from(*m)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_selector_valid() -> std::result::Result<(), String> {
        let tests = [
            (
                "foo{}",
                VectorSelector::new(Some("foo".to_string()), LabelMatchers::empty())?,
            ),
            // ("foo{ }", vec![]),
            // ("foo {   }", vec![]),
            // ("foo   {   }  ", vec![]),
            // ("foo{} or", vec![]),
            // (
            //     r#"{foo!~"123 qux"}"#,
            //     vec![("foo", MatchOp::NeqRe, "123 qux")],
            // ),
            // (
            //     r#"{foo!~"123 qux",bar="42"}"#,
            //     vec![
            //         ("foo", MatchOp::NeqRe, "123 qux"),
            //         ("bar", MatchOp::Eql, "42"),
            //     ],
            // ),
            // (r#"{ foo="bar"}"#, vec![("foo", MatchOp::Eql, "bar")]),
            // (r#"{  foo="bar"}"#, vec![("foo", MatchOp::Eql, "bar")]),
            // (r#"{foo="bar",}"#, vec![("foo", MatchOp::Eql, "bar")]),
            // (r#"{foo="bar" ,}"#, vec![("foo", MatchOp::Eql, "bar")]),
            // (r#"{foo="bar"  ,}"#, vec![("foo", MatchOp::Eql, "bar")]),
            // (r#"{foo="bar"  , }"#, vec![("foo", MatchOp::Eql, "bar")]),
            // (r#"{foo="bar"  ,  }"#, vec![("foo", MatchOp::Eql, "bar")]),
            // (
            //     r#"{foo="bar",qux="123"}"#,
            //     vec![("foo", MatchOp::Eql, "bar"), ("qux", MatchOp::Eql, "123")],
            // ),
            // (
            //     r#"{foo="bar", qux="123"}"#,
            //     vec![("foo", MatchOp::Eql, "bar"), ("qux", MatchOp::Eql, "123")],
            // ),
            // (
            //     r#"{foo="bar" , qux="123"}"#,
            //     vec![("foo", MatchOp::Eql, "bar"), ("qux", MatchOp::Eql, "123")],
            // ),
            // (
            //     r#"{foo="bar", qux="123", abc="xyz"}"#,
            //     vec![
            //         ("foo", MatchOp::Eql, "bar"),
            //         ("qux", MatchOp::Eql, "123"),
            //         ("abc", MatchOp::Eql, "xyz"),
            //     ],
            // ),
            // (
            //     r#"{foo="bar", qux="123" , abc="xyz"}"#,
            //     vec![
            //         ("foo", MatchOp::Eql, "bar"),
            //         ("qux", MatchOp::Eql, "123"),
            //         ("abc", MatchOp::Eql, "xyz"),
            //     ],
            // ),
            // (r#"{ foo ="bar"}"#, vec![("foo", MatchOp::Eql, "bar")]),
            // (r#"{ foo= "bar"}"#, vec![("foo", MatchOp::Eql, "bar")]),
            // (r#"{ foo = "bar"}"#, vec![("foo", MatchOp::Eql, "bar")]),
            // (
            //     r#"{    foo   =    "bar",   qux    =   "123"    }"#,
            //     vec![("foo", MatchOp::Eql, "bar"), ("qux", MatchOp::Eql, "123")],
            // ),
        ];

        for (input, expected_selector) in tests.iter() {
            let actual_selector =
                match vector_selector(Span::new(&input)).map_err(|e| ParseError::from(e))? {
                    (_, ParseResult::Complete(s)) => s,
                    _ => unreachable!(),
                };
            assert_eq!(&actual_selector, expected_selector);
        }
        Ok(())
    }

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
                ParseResult::Complete(LabelMatchers::new(
                    expected_matchers
                        .iter()
                        .map(|(l, o, v)| LabelMatcher::new(*l, *o, *v))
                        .collect()
                ))
            );
        }
        Ok(())
    }

    #[test]
    fn test_label_matchers_partial() -> std::result::Result<(), String> {
        let tests = [
            ("{", "", (1, 1), r#"identifier or "}""#),
            ("{123", "123", (1, 1), r#"identifier or "}""#),
            ("{,}", ",}", (1, 1), r#"identifier or "}""#),
            (
                r#"{foo!~"123 qux",,}"#,
                r#",}"#,
                (1, 16),
                r#"identifier or "}""#,
            ),
            ("{foo", "", (1, 4), r#"one of "=", "!=", "=~", "!~""#),
            (
                r#"{foo="bar",f12=}"#,
                r#"}"#,
                (1, 15),
                "label value as string literal",
            ),
            (
                r#"{foo="bar",baz=~"42",qux!}"#,
                r#"!}"#,
                (1, 24),
                r#"one of "=", "!=", "=~", "!~""#,
            ),
        ];

        for &(input, unexpected, error_pos, expected) in tests.iter() {
            let (rest, matchers) =
                label_matchers(Span::new(input)).map_err(|e| ParseError::from(e))?;
            assert_eq!(matchers, ParseResult::Partial("label matching", expected));
            assert_eq!(*rest, unexpected);
            assert_eq!((rest.location_line(), rest.location_offset()), error_pos);
        }
        Ok(())
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
                    (_, ParseResult::Complete(m)) => m,
                    (_, ParseResult::Partial(_, _)) => panic!("oops"),
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
                ParseResult::Complete(LabelMatcher::new(label, op, value))
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

        for &(input, output, expected, error_pos) in tests.iter() {
            match label_matcher(Span::new(input)) {
                Ok((span, ParseResult::Partial(wherein, exp))) => {
                    assert_eq!(*span, output);
                    assert_eq!((span.location_line(), span.location_offset()), error_pos);
                    assert_eq!(exp, expected);
                    assert_eq!(wherein, "label matching");
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
