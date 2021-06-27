use std::convert::TryFrom;
use std::time::Duration;

use nom::{branch::alt, bytes::complete::tag, character::complete::char};

use super::ast::{Expr, VectorSelector};
use crate::common::parser::{
    duration, label_identifier, maybe_lpadded, metric_identifier, separated_list, string_literal,
    IResult, ParseError, Span,
};
use crate::model::{LabelMatcher, MatchOp};

pub(super) fn expr_vector_selector(input: Span) -> IResult<Expr> {
    let (rest, vs) = vector_selector(input)?;
    Ok((rest, Expr::VectorSelector(vs)))
}

pub(super) fn vector_selector(input: Span) -> IResult<VectorSelector> {
    //   metric_identifier label_matchers
    // | metric_identifier
    // | label_matchers

    let (rest, metric) = match metric_identifier(input) {
        Ok((r, m)) => (r, Some(m)),
        Err(_) => (input, None),
    };

    let (rest, matchers) = match maybe_lpadded(label_matchers)(rest) {
        Ok((r, ms)) => (r, ms),
        Err(nom::Err::Error(_)) if metric.is_some() => (rest, vec![]),
        Err(e) => return Err(e),
    };

    let (rest, duration) = match maybe_lpadded(range_duration)(rest) {
        Ok((r, d)) => (r, Some(d)),
        Err(nom::Err::Error(_)) => (rest, None),
        Err(e) => return Err(e),
    };

    let selector = VectorSelector::new(metric, matchers, duration)
        .map_err(|e| nom::Err::Failure(ParseError::new(e.to_string(), input)))?;
    Ok((rest, selector))
}

fn label_matchers(input: Span) -> IResult<Vec<LabelMatcher>> {
    //   LEFT_BRACE label_match_list RIGHT_BRACE
    // | LEFT_BRACE label_match_list COMMA RIGHT_BRACE
    // | LEFT_BRACE RIGHT_BRACE

    separated_list(
        '{',
        '}',
        ',',
        label_matcher,
        "label matching",
        r#"identifier or "}""#,
    )(input)
}

fn label_matcher(input: Span) -> IResult<LabelMatcher> {
    // IDENTIFIER match_op STRING

    let (rest, label) = label_identifier(input)?;

    let (rest, op) = match maybe_lpadded(match_op)(rest) {
        Ok(v) => v,
        Err(_) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "label matching",
                r#"one of "=", "!=", "=~", "!~""#,
                rest,
            )))
        }
    };

    let (rest, value) = match maybe_lpadded(string_literal)(rest) {
        Ok(v) => v,
        Err(_) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "label matching",
                "label value as string literal",
                rest,
            )))
        }
    };

    let matcher = LabelMatcher::new(label, op, value)
        .map_err(|e| nom::Err::Failure(ParseError::new(e.to_string(), input)))?;

    Ok((rest, matcher))
}

fn match_op(input: Span) -> IResult<MatchOp> {
    let (rest, m) = alt((tag("=~"), tag("!~"), tag("!="), tag("=")))(input)?;
    Ok((
        rest,
        MatchOp::try_from(*m.fragment()).expect("unreachable!"),
    ))
}

fn range_duration(input: Span) -> IResult<Duration> {
    let (rest, _) = char('[')(input)?;

    let (rest, d) = match duration(rest) {
        Ok((rest, d)) => (rest, d),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "range vector selector",
                "duration literal",
                rest,
            )));
        }
        Err(e) => return Err(e),
    };

    let (rest, _) = match char(']')(rest) {
        Ok((rest, _)) => (rest, '_'),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "range vector selector",
                "]",
                rest,
            )));
        }
        Err(e) => return Err(e),
    };

    Ok((rest, d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_selector_valid() {
        #[rustfmt::skip]
        let tests = &[
            (r#"foo"#, Some("foo"), None, vec![]),
            (r#"foo[1h5m]"#, Some("foo"), Some(Duration::from_secs(3900)), vec![]),
            (r#"foo or bar"#, Some("foo"), None, vec![]),
            (r#"foo{}"#, Some("foo"), None, vec![]),
            (r#"foo {}"#, Some("foo"), None, vec![]),
            (r#"foo {}[5ms]"#, Some("foo"), Some(Duration::from_millis(5)), vec![]),
            (r#"foo {}  [1m3s]"#, Some("foo"), Some(Duration::from_secs(63)), vec![]),
            (r#"foo  {   }"#, Some("foo"), None, vec![]),
            (r#"{__name__="foo"}"#, None, None, vec![("__name__", "=", "foo")]),
            (r#"{__name__=~"foo"}"#, None, None, vec![("__name__", "=~", "foo")]),
            (r#"{__name__=~"foo",__name__=~"bar"}"#, None, None, vec![("__name__", "=~", "foo"), ("__name__", "=~", "bar")]),
            (r#"foo{name=~"bar"}"#, Some("foo"), None, vec![("name", "=~", "bar")]),
        ];

        for (input, metric, duration, labels) in tests {
            let actual_selector = match vector_selector(Span::new(input)) {
                Ok((_, s)) => s,
                Err(e) => panic!("Got error {} while testing input {}", e, input),
            };
            assert_eq!(
                VectorSelector::new(*metric, _matchers(labels), *duration).expect("bad test case"),
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
            (r#"{}"#, "1:0: parse error: vector selector must contain at least one non-empty matcher", (1, 0)),
            (r#"{foo=""}"#, "1:0: parse error: vector selector must contain at least one non-empty matcher", (1, 0)),
            (r#"{foo=~".*"}"#, "1:0: parse error: vector selector must contain at least one non-empty matcher", (1, 0)),
            (r#"{foo!~".+"}"#, "1:0: parse error: vector selector must contain at least one non-empty matcher", (1, 0)),
            (r#"{foo!="bar"}"#, "1:0: parse error: vector selector must contain at least one non-empty matcher", (1, 0)),
            (r#"foo{__name__="foo"}"#, "1:0: parse error: potentially ambiguous metric name match", (1, 0)),
            (r#"foo{__name__="bar"}"#, "1:0: parse error: potentially ambiguous metric name match", (1, 0)),
        ];

        for &(input, err_msg, err_pos) in &tests {
            let err = match vector_selector(Span::new(input)) {
                Ok((_, s)) => {
                    panic!("Got result {:?} while testing input {}", s, input)
                }
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
            assert_eq!(_matchers(expected_matchers), actual_matchers);
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
            let err = match label_matchers(Span::new(input)) {
                Err(nom::Err::Failure(e)) => e,
                _ => panic!("unexpected result"),
            };
            assert_eq!(
                ParseError::partial("label matching", expected, *err.span()).message(),
                err.message()
            );
            assert_eq!(unexpected, **err.span());
            assert_eq!(error_pos, (err.line(), err.offset()));
        }
        Ok(())
    }

    #[test]
    fn test_label_matchers_invalid() -> std::result::Result<(), ParseError<'static>> {
        #[rustfmt::skip]
        let tests = [
            (r#"{foo=~"*"}"#, (1, 1), "1:1: parse error: regex parse error:\n    ^(?:*)$\n        ^\nerror: repetition operator missing expression"),
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
    fn test_label_matcher_valid() -> std::result::Result<(), ParseError<'static>> {
        #[rustfmt::skip]
        let tests = [
            (r#"foo="bar""#, ("foo", "=", "bar")),
            (r#"foo!~"123 qux""#, ("foo", "!~", "123 qux")),
        ];

        for &(input, expected) in &tests {
            let (_, actual) = label_matcher(Span::new(input))?;
            assert_eq!(_matcher(expected), actual);
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
            let err = match label_matcher(Span::new(input)) {
                Err(nom::Err::Failure(e)) => e,
                Ok(res) => panic!("nom::Err::Failure expected but found {:#?}", res),
                Err(err) => panic!("nom::Err::Failure expected but found {:#?}", err),
            };
            assert_eq!(**err.span(), output);
            assert_eq!((err.line(), err.offset()), error_pos);
            assert_eq!(
                ParseError::partial("label matching", expected, *err.span()).message(),
                err.message()
            );
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
