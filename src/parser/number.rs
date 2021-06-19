use nom::number::complete::double;

use super::ast::Expr;
use super::result::{IResult, Span};

pub(super) fn expr_number_literal(input: Span) -> IResult<Expr> {
    let (rest, n) = number_literal(input)?;
    Ok((rest, Expr::NumberLiteral(n)))
}

pub(super) fn number_literal(input: Span) -> IResult<f64> {
    let (rest, n) = double(input)?;
    Ok((rest, n))
}

#[cfg(test)]
mod tests {
    use super::super::expr::expr;
    use super::super::result::ParseError;
    use super::*;

    #[test]
    fn test_valid_expressions_ex() -> std::result::Result<(), nom::Err<ParseError<'static>>> {
        use Expr::*;

        let tests = [
            ("1", NumberLiteral(1.0)),
            ("1.", NumberLiteral(1.0)),
            (".1", NumberLiteral(0.1)),
            ("2e-5", NumberLiteral(0.00002)),
            ("Inf", NumberLiteral(f64::INFINITY)),
            ("+Inf", NumberLiteral(f64::INFINITY)),
            ("-Inf", NumberLiteral(f64::NEG_INFINITY)),
        ];

        for (input, expected_expr) in &tests {
            let (_, actual_expr) = expr(None)(Span::new(input))?;
            assert_eq!(expected_expr, &actual_expr, "while parsing {}", input);
        }
        Ok(())
    }
}
