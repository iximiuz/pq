use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1},
    combinator::recognize,
    multi::{many0, separated_list1},
    sequence::pair,
};

use super::ast::{LabelMatcher, LabelMatchers, MatchOp, VectorSelector};
use super::error::ParseError;
use super::result::{unexpected, IResult, ParseResult, Span};
use super::string::string_literal;

pub fn vector_selector(input: Span) -> IResult<VectorSelector> {
    // metric_identifier label_matchers | metric_identifier | label_matchers
    let (rest, matchers) = label_matchers(input)?;
    Ok((rest, VectorSelector::new(None, matchers).unwrap())) // TODO: handle unwrap
}

fn label_matchers(input: Span) -> IResult<LabelMatchers> {
    println!("label_matchers({})", *input);
    // LEFT_BRACE label_match_list RIGHT_BRACE
    //   | LEFT_BRACE label_match_list COMMA RIGHT_BRACE
    //   | LEFT_BRACE RIGHT_BRACE

    let (rest, m) = alt((tag("{}"), tag("{")))(input)?;
    if *m == "{}" {
        return Ok((rest, LabelMatchers::empty()));
    }

    let (rest, matchers) = match label_match_list(rest)? {
        (r, ParseResult::Success(m)) => (r, m),
        (_, ParseResult::Partial(span, diag)) => {
            return Err(nom::Err::Error(ParseError::new(
                format!(
                    "unexpected {} in label matching, expected {}",
                    unexpected(*span),
                    diag,
                ),
                span.location_line(),
                span.location_offset(),
            )));
        }
    };

    let (rest, _) = alt((tag(",}"), tag("}")))(rest)?;

    Ok((rest, LabelMatchers::new(matchers)))
}

/// Parses a non-empty list of label matches separated by a comma.
/// No trailing commas allowed.
fn label_match_list(input: Span) -> IResult<ParseResult<Vec<LabelMatcher>>> {
    println!("label_match_list({})", *input);
    // label_match_list COMMA label_matcher | label_matcher

    let (rest, matches) = separated_list1(tag(","), label_matcher)(input)?;
    let mut matchers = vec![];

    for m in matches.into_iter() {
        match m {
            ParseResult::Success(m) => matchers.push(m),
            ParseResult::Partial(span, diag) => {
                return Ok((span, ParseResult::Partial(span, diag)))
            }
        };
    }

    Ok((rest, ParseResult::Success(matchers)))
}

/// label_matcher actually never returns IResult::Err.
/// Instead of an error, a partial ParseResult::Partial is returned.
fn label_matcher(input: Span) -> IResult<ParseResult<LabelMatcher>> {
    println!("label_matcher({})", *input);
    // IDENTIFIER match_op STRING

    let (rest, label) = match label_identifier(input) {
        Ok(v) => v,
        Err(_) => {
            println!("label_matcher_error - label");
            return Ok((input, ParseResult::Partial(input, r#"identifier or "}""#)));
        }
    };

    let (rest, op) = match match_op(rest) {
        Ok(v) => v,
        Err(_) => {
            println!("label_matcher_error - op");
            return Ok((
                rest,
                ParseResult::Partial(rest, r#"one of "=", "!=", "=~", "!~"""#),
            ));
        }
    };

    let (rest, value) = match string_literal(rest) {
        Ok(v) => v,
        Err(_) => {
            println!("label_matcher_error - value");
            return Ok((rest, ParseResult::Partial(rest, "string label value")));
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
        let err = label_matchers(Span::new(r#"{foo="bar",f12=}"#));
        println!("{:#?}", err);
        assert!(false);
        // assert!(label_matchers("{,}").is_err());
        // assert!(label_matchers("{foo!~\"123 qux\",,}").is_err());
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
                ParseResult::Success(LabelMatcher::new(label, op, value))
            );
        }
        Ok(())
    }

    #[test]
    fn test_label_matcher_invalid() {
        let tests: &[(&str, &str, (u32, usize))] = &[
            (
                "",
                r#"unexpected EOF in label matching, expected identifier or "}""#,
                (1, 0),
            ),
            (
                "123",
                r#"unexpected "123" in label matching, expected identifier or "}""#,
                (1, 0),
            ),
            (
                "foo",
                r#"unexpected EOF in label matching, expected one of "=", "!=", "=~", "!~""#,
                (1, 3),
            ),
            (
                "foo!",
                r#"unexpected "!" in label matching, expected one of "=", "!=", "=~", "!~""#,
                (1, 3),
            ),
            (
                "foo!=",
                r#"unexpected EOF in label matching, expected string label value"#,
                (1, 5),
            ),
            (
                "foo!=123",
                r#"unexpected "123" in label matching, expected string label value"#,
                (1, 5),
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
