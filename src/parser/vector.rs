use std::convert::TryFrom;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, char},
    combinator::recognize,
    multi::{many0, separated_list1},
    sequence::pair,
};

use super::ast::VectorSelector;
use super::common::{maybe_lpadded, maybe_padded};
use super::result::{IResult, ParseError, ParseResult, Span};
use super::string::string_literal;
use crate::model::labels::{LabelMatcher, MatchOp};

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
        Err(nom::Err::Error(_)) if metric.is_some() => (rest, vec![]),
        Err(e) => return Err(e),
    };

    let selector = VectorSelector::new(metric, matchers)
        .map_err(|e| nom::Err::Failure(ParseError::new(e.to_string(), input)))?;
    Ok((rest, ParseResult::Complete(selector)))
}

fn label_matchers(input: Span) -> IResult<ParseResult<Vec<LabelMatcher>>> {
    //   LEFT_BRACE label_match_list RIGHT_BRACE
    // | LEFT_BRACE label_match_list COMMA RIGHT_BRACE
    // | LEFT_BRACE RIGHT_BRACE

    let (rest, _) = char('{')(input)?;

    let (rest, matchers) = match maybe_lpadded(label_match_list)(rest) {
        Ok((r, ParseResult::Partial(w, e))) => return Ok((r, ParseResult::Partial(w, e))),
        Ok((r, ParseResult::Complete(m))) => (r, m),
        Err(nom::Err::Error(_)) => (rest, vec![]),
        Err(e) => return Err(e),
    };

    // Chop off a possible trailing comma, but only matchers list is not empty.
    let (rest, _) = match matchers.len() {
        0 => (rest, '_'),
        _ => maybe_lpadded(char(','))(rest).unwrap_or((rest, '_')),
    };

    Ok(match maybe_lpadded(char('}'))(rest) {
        Ok((r, _)) => (r, ParseResult::Complete(matchers)),
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

    let matcher = LabelMatcher::new(label, op, value)
        .map_err(|e| nom::Err::Failure(ParseError::new(e.to_string(), input)))?;

    Ok((rest, ParseResult::Complete(matcher)))
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
    Ok((
        rest,
        MatchOp::try_from(*m.fragment()).expect("unreachable!"),
    ))
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
    fn test_vector_selector_valid() {
        #[rustfmt::skip]
        let tests = [
            (r#"foo"#, Some("foo"), vec![]),
            (r#"foo or bar"#, Some("foo"), vec![]),
            (r#"foo{}"#, Some("foo"), vec![]),
            (r#"foo {}"#, Some("foo"), vec![]),
            (r#"foo  {   }"#, Some("foo"), vec![]),
            (r#"{__name__="foo"}"#, None, vec![("__name__", "=", "foo")]),
            (r#"{__name__=~"foo"}"#, None, vec![("__name__", "=~", "foo")]),
            (r#"{__name__=~"foo",__name__=~"bar"}"#, None, vec![("__name__", "=~", "foo"), ("__name__", "=~", "bar")]),
            (r#"foo{name=~"bar"}"#, Some("foo"), vec![("name", "=~", "bar")]),
        ];

        for (input, metric, labels) in &tests {
            let actual_selector = match vector_selector(Span::new(input)) {
                Ok((_, ParseResult::Complete(s))) => s,
                Ok((_, ParseResult::Partial(u, w))) => panic!(
                    "Got partial result {}/{} while testing input {}",
                    u, w, input
                ),
                Err(e) => panic!("Got error {} while testing input {}", e, input),
            };
            assert_eq!(
                VectorSelector::new(*metric, _matchers(labels)).expect("bad test case"),
                actual_selector,
                "while testing input {}",
                input,
            );
        }
    }

    #[test]
    fn test_vector_selector_invalid() {
        #[rustfmt::skip]
        let tests = [
            (r#"{}"#, "vector selector must contain at least one non-empty matcher", (1, 0)),
            (r#"{foo=""}"#, "vector selector must contain at least one non-empty matcher", (1, 0)),
            (r#"{foo=~".*"}"#, "vector selector must contain at least one non-empty matcher", (1, 0)),
            (r#"{foo!~".+"}"#, "vector selector must contain at least one non-empty matcher", (1, 0)),
            (r#"{foo!="bar"}"#, "vector selector must contain at least one non-empty matcher", (1, 0)),
            (r#"foo{__name__="foo"}"#, "potentially ambiguous metric name match", (1, 0)),
            (r#"foo{__name__="bar"}"#, "potentially ambiguous metric name match", (1, 0)),
        ];

        for &(input, err_msg, err_pos) in &tests {
            let err = match vector_selector(Span::new(input)) {
                Ok((_, ParseResult::Complete(s))) => {
                    panic!("Got complete result {:?} while testing input {}", s, input)
                }
                Ok((_, ParseResult::Partial(u, w))) => panic!(
                    "Got partial result {}/{} while testing input {}",
                    u, w, input
                ),
                Err(nom::Err::Error(e)) => e,
                Err(nom::Err::Failure(e)) => e,
                _ => unreachable!(),
            };
            assert_eq!(err_msg, err.message());
            assert_eq!(err_pos, (err.line(), err.offset()));
        }
    }

    #[test]
    fn test_label_matchers_valid() -> std::result::Result<(), ParseError<'static>> {
        #[rustfmt::skip]
        let tests = [
            (r#"{}"#, vec![]),
            (r#"{ }"#, vec![]),
            (r#"{   }"#, vec![]),
            (r#"{   }  "#, vec![]),
            (r#"{} or"#, vec![]),
            (r#"{foo!~"123 qux"}"#, vec![("foo", "!~", "123 qux")]),
            (r#"{foo!~"123 qux",bar="42"}"#, vec![("foo", "!~", "123 qux"), ("bar", "=", "42")]),
            (r#"{ foo="bar"}"#, vec![("foo", "=", "bar")]),
            (r#"{  foo="bar"}"#, vec![("foo", "=", "bar")]),
            (r#"{foo="bar",}"#, vec![("foo", "=", "bar")]),
            (r#"{foo="bar" ,}"#, vec![("foo", "=", "bar")]),
            (r#"{foo="bar"  ,}"#, vec![("foo", "=", "bar")]),
            (r#"{foo="bar"  , }"#, vec![("foo", "=", "bar")]),
            (r#"{foo="bar"  ,  }"#, vec![("foo", "=", "bar")]),
            (r#"{foo="bar",qux="123"}"#, vec![("foo", "=", "bar"), ("qux", "=", "123")]),
            (r#"{foo="bar", qux="123"}"#, vec![("foo", "=", "bar"), ("qux", "=", "123")]),
            (r#"{foo="bar" , qux="123"}"#, vec![("foo", "=", "bar"), ("qux", "=", "123")]),
            (r#"{foo="bar", qux="123", abc="xyz"}"#, vec![("foo", "=", "bar"), ("qux", "=", "123"), ("abc", "=", "xyz")]),
            (r#"{foo="bar", qux="123" , abc="xyz"}"#, vec![("foo", "=", "bar"), ("qux", "=", "123"), ("abc", "=", "xyz")]),
            (r#"{ foo ="bar"}"#, vec![("foo", "=", "bar")]),
            (r#"{ foo= "bar"}"#, vec![("foo", "=", "bar")]),
            (r#"{ foo = "bar"}"#, vec![("foo", "=", "bar")]),
            (r#"{    foo   =    "bar",   qux    =   "123"    }"#, vec![("foo", "=", "bar"), ("qux", "=", "123")]),
        ];

        for (input, expected_matchers) in &tests {
            let (_, actual_matchers) = label_matchers(Span::new(input))?;
            assert_eq!(
                ParseResult::Complete(_matchers(expected_matchers)),
                actual_matchers
            );
        }
        Ok(())
    }

    #[test]
    fn test_label_matchers_partial() -> std::result::Result<(), ParseError<'static>> {
        #[rustfmt::skip]
        let tests = [
            (r#"{"#, "", (1, 1), r#"identifier or "}""#),
            (r#"{123"#, "123", (1, 1), r#"identifier or "}""#),
            (r#"{,}"#, ",}", (1, 1), r#"identifier or "}""#),
            (r#"{foo!~"123 qux",,}"#, r#",}"#, (1, 16), r#"identifier or "}""#),
            (r#"{foo"#, "", (1, 4), r#"one of "=", "!=", "=~", "!~""#),
            (r#"{foo="bar",f12=}"#, r#"}"#, (1, 15), "label value as string literal"),
            (r#"{foo="bar",baz=~"42",qux!}"#, r#"!}"#, (1, 24), r#"one of "=", "!=", "=~", "!~""#),
        ];

        for &(input, unexpected, error_pos, expected) in &tests {
            let (rest, matchers) = label_matchers(Span::new(input))?;
            assert_eq!(matchers, ParseResult::Partial("label matching", expected));
            assert_eq!(*rest, unexpected);
            assert_eq!((rest.location_line(), rest.location_offset()), error_pos);
        }
        Ok(())
    }

    #[test]
    fn test_label_matchers_invalid() -> std::result::Result<(), ParseError<'static>> {
        #[rustfmt::skip]
        let tests = [
            (r#"{foo=~"*"}"#, (1, 1), "regex parse error:\n    ^(?:*)$\n        ^\nerror: repetition operator missing expression"),
        ];

        for &(input, err_pos, err_msg) in &tests {
            match label_matchers(Span::new(input)) {
                Err(nom::Err::Failure(err)) => {
                    assert_eq!(err_msg, err.message());
                    assert_eq!(err_pos, (err.line(), err.offset()));
                }
                Err(err) => panic!("nom::Err::Failure expected but found {:#?}", err),
                Ok(res) => panic!("nom::Err::Failure expected but found {:#?}", res),
            };
        }
        Ok(())
    }

    #[test]
    fn test_label_match_list_valid() -> std::result::Result<(), ParseError<'static>> {
        #[rustfmt::skip]
        let tests = [
            (r#"foo!~"123 qux""#, vec![("foo", "!~", "123 qux")]),
            (r#"foo!~"123 qux","#, vec![("foo", "!~", "123 qux")]),
            (r#"foo!~"123 qux",bar="42""#, vec![("foo", "!~", "123 qux"), ("bar", "=", "42")]),
        ];

        for (input, expected_matchers) in &tests {
            let actual_matchers = match label_match_list(Span::new(&input))? {
                (_, ParseResult::Complete(m)) => m,
                (_, ParseResult::Partial(u, w)) => panic!(
                    "Got partial result {}/{} while testing input {}",
                    u, w, input
                ),
            };

            assert_eq!(actual_matchers.len(), expected_matchers.len());
            for (actual, expected) in actual_matchers.iter().zip(expected_matchers.iter()) {
                assert_eq!(&_matcher(*expected), actual);
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
    fn test_label_matcher_valid() -> std::result::Result<(), ParseError<'static>> {
        #[rustfmt::skip]
        let tests = [
            (r#"foo="bar""#, ("foo", "=", "bar")),
            (r#"foo!~"123 qux""#, ("foo", "!~", "123 qux")),
        ];

        for &(input, expected) in &tests {
            let (_, actual) = label_matcher(Span::new(input))?;
            assert_eq!(ParseResult::Complete(_matcher(expected)), actual);
        }
        Ok(())
    }

    #[test]
    fn test_label_matcher_partial() {
        #[rustfmt::skip]
        let tests = [
            ("foo!", "!", r#"one of "=", "!=", "=~", "!~""#, (1, 3)),
            ("foo!=", "", r#"label value as string literal"#, (1, 5)),
            ("foo!= ", " ", r#"label value as string literal"#, (1, 5)),
            ("foo!=,", ",", r#"label value as string literal"#, (1, 5)),
            ("foo!=123", "123", r#"label value as string literal"#, (1, 5)),
        ];

        for &(input, output, expected, error_pos) in &tests {
            match label_matcher(Span::new(input)) {
                Ok((span, ParseResult::Partial(wherein, exp))) => {
                    assert_eq!(*span, output);
                    assert_eq!((span.location_line(), span.location_offset()), error_pos);
                    assert_eq!(expected, exp);
                    assert_eq!("label matching", wherein);
                }
                Ok(res) => panic!("ParseResult::Partial expected but found {:#?}", res),
                Err(err) => panic!("ParseResult::Partial expected but found {:#?}", err),
            };
        }
    }

    #[test]
    fn test_label_matcher_invalid() {
        // We don't care about actual error, just the fact that it errored.
        let tests = ["", ",", "123", "1foo=", r#"foo=~"*""#];

        for input in &tests {
            let res = label_matcher(Span::new(input));
            assert!(res.is_err(), "Error expected but found {:#?}", res);
        }
    }

    #[test]
    fn test_label_identifier_valid() -> std::result::Result<(), ParseError<'static>> {
        #[rustfmt::skip]
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

        for &(input, expected_label) in &tests {
            let (_, actual_label) = label_identifier(Span::new(input))?;
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
    fn test_match_op_valid() -> std::result::Result<(), ParseError<'static>> {
        #[rustfmt::skip]
        let tests = [
            (r#"="foo""#, r#""foo""#, MatchOp::Eql),
            (r#"!="foo""#, r#""foo""#, MatchOp::Neq),
            (r#"=~"foo""#, r#""foo""#, MatchOp::EqlRe),
            (r#"!~"foo""#, r#""foo""#, MatchOp::NeqRe),
        ];

        for &(input, expected_remainder, expected_match_op) in &tests {
            let (actual_remainder, actual_match_op) = match_op(Span::new(input))?;
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

    fn _matcher((label, op, value): (&str, &str, &str)) -> LabelMatcher {
        LabelMatcher::new(label, MatchOp::try_from(op).unwrap(), value).expect("bad test data")
    }

    fn _matchers(matchers: &[(&str, &str, &str)]) -> Vec<LabelMatcher> {
        matchers.iter().map(|&t| _matcher(t)).collect()
    }
}
