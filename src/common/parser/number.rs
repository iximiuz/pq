use nom::number::complete::double;

use super::result::{IResult, Span};

pub fn number_literal(input: Span) -> IResult<f64> {
    let (rest, n) = double(input)?;
    Ok((rest, n))
}

#[cfg(test)]
mod tests {
    use super::super::result::ParseError;
    use super::*;

    #[test]
    fn test_valid_number_literal() -> std::result::Result<(), nom::Err<ParseError<'static>>> {
        let tests = [
            ("1", 1.0),
            ("1.", 1.0),
            (".1", 0.1),
            ("2e-5", 0.00002),
            ("Inf", f64::INFINITY),
            ("+Inf", f64::INFINITY),
            ("-Inf", f64::NEG_INFINITY),
        ];

        for (input, expected) in &tests {
            let (_, actual) = number_literal(Span::new(input))?;
            assert_eq!(expected, &actual, "while parsing {}", input);
        }
        Ok(())
    }
}
