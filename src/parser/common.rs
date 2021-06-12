use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, char as nom_char, multispace0},
    combinator::recognize,
    multi::{many0, separated_list1},
    sequence::{delimited, pair, preceded, terminated},
};

use super::result::{IResult, ParseError, Span};

pub fn label_identifier(input: Span) -> IResult<String> {
    // [a-zA-Z_][a-zA-Z0-9_]*
    let (rest, m) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))(input)?;
    Ok((rest, String::from(*m.fragment())))
}

pub fn metric_identifier(input: Span) -> IResult<String> {
    // [a-zA-Z_:][a-zA-Z0-9_:]*
    let (rest, m) = recognize(pair(
        alt((alpha1, tag("_"), tag(":"))),
        many0(alt((alphanumeric1, tag("_"), tag(":")))),
    ))(input)?;
    Ok((rest, String::from(*m)))
}

pub fn separated_list<'a, F, O>(
    opener: char,
    closer: char,
    sep: char,
    element_parser: F,
    wherein: &'static str,
    expected: &'static str,
) -> impl FnMut(Span<'a>) -> IResult<Vec<O>>
where
    F: Clone + Copy + FnMut(Span<'a>) -> IResult<O>,
{
    // |  OPENER element_list CLOSER
    // |  OPENER element_list SEP CLOSER
    // |  OPENER CLOSER

    move |input: Span| {
        let (rest, _) = nom_char(opener)(input)?;

        let (rest, elements) =
            match separated_list1(nom_char(sep), maybe_padded(element_parser))(rest) {
                Ok((r, ms)) => (r, ms),
                Err(nom::Err::Error(_)) => (rest, vec![]),
                Err(e) => return Err(e),
            };

        // Chop off a possible trailing separator, but only if element list is not empty.
        let (rest, _) = match elements.len() {
            0 => (rest, '_'),
            _ => maybe_lpadded(nom_char(sep))(rest).unwrap_or((rest, '_')),
        };

        match maybe_lpadded(nom_char(closer))(rest) {
            Ok((r, _)) => Ok((r, elements)),
            Err(_) => Err(nom::Err::Failure(ParseError::partial(
                wherein, expected, rest,
            ))),
        }
    }
}

pub fn maybe_padded<'a, F, O>(f: F) -> impl FnMut(Span<'a>) -> IResult<O>
where
    F: FnMut(Span<'a>) -> IResult<O>,
{
    delimited(multispace0, f, multispace0)
}

pub fn maybe_lpadded<'a, F, O>(f: F) -> impl FnMut(Span<'a>) -> IResult<O>
where
    F: FnMut(Span<'a>) -> IResult<O>,
{
    preceded(multispace0, f)
}

#[allow(dead_code)]
pub fn maybe_rpadded<'a, F, O>(f: F) -> impl FnMut(Span<'a>) -> IResult<O>
where
    F: FnMut(Span<'a>) -> IResult<O>,
{
    terminated(f, multispace0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_separated_list_valid() -> std::result::Result<(), ParseError<'static>> {
        #[rustfmt::skip]
        let tests = [
            ("()", vec![]),
            ("( )", vec![]),
            ("(  )", vec![]),
            ("(   )", vec![]),
            ("(foo)", vec!["foo"]),
            ("( foo)", vec!["foo"]),
            ("(foo )", vec!["foo"]),
            ("( foo )", vec!["foo"]),
            ("(foo,bar)", vec!["foo", "bar"]),
            ("(foo, bar)", vec!["foo", "bar"]),
            ("(foo,  bar   , )", vec!["foo", "bar"]),
            ("(foo, bar, baz)", vec!["foo", "bar", "baz"]),
        ];

        for (input, expected_elements) in &tests {
            let (_, actual_elements) =
                separated_list('(', ')', ',', label_identifier, "label list", "label")(Span::new(
                    &input,
                ))?;

            assert_eq!(&actual_elements, expected_elements);
        }
        Ok(())
    }

    #[test]
    fn test_separated_list_invalid() {
        #[rustfmt::skip]
        let tests = [
            ("(", vec![]),
            (")", vec![]),
            ("(,)", vec![]),
            ("(foo }", vec!["foo"]),
            ("( foo bar )", vec!["foo"]),
        ];

        for (input, _expected_elements) in &tests {
            if let Ok(res) = separated_list('(', ')', ',', label_identifier, "label list", "label")(
                Span::new(&input),
            ) {
                panic!(
                    "expected error but found {:?} while testing '{}'",
                    res, *input
                );
            }
        }
    }
}
