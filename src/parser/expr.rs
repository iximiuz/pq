use nom::{branch::alt, character::complete::char, number::complete::double, sequence::pair};

use super::ast::{BinaryOp, Expr, Precedence, UnaryOp};
use super::common::maybe_lpadded;
use super::result::{IResult, ParseResult, Span};
use super::vector::vector_selector;

pub fn expr<'a>(min_prec: Precedence) -> impl FnMut(Span<'a>) -> IResult<ParseResult<Expr>> {
    move |input: Span| {
        let (mut rest, mut lhs) =
            match alt((expr_number_literal, expr_unary, expr_vector_selector))(input)? {
                (r, ParseResult::Complete(e)) => (r, e),
                (r, ParseResult::Partial(w, u)) => return Ok((r, ParseResult::Partial(w, u))),
            };

        // The rest is dealing with the left-recursive grammar.
        // E.g.  expr = unary_expr | vector_selector | binary_expr ...
        // where binary_expr = expr <OP> expr

        while *rest != "" {
            let (tmp_rest, op) = maybe_lpadded(binary_op)(rest)?;
            if op.precedence() <= min_prec {
                break;
            }
            rest = tmp_rest;

            let (tmp_rest, rhs) = match maybe_lpadded(expr(op.precedence()))(rest) {
                Ok((r, ParseResult::Complete(e))) => (r, e),
                Ok((r, ParseResult::Partial(w, u))) => return Ok((r, ParseResult::Partial(w, u))),
                Err(nom::Err::Error(_)) => {
                    return Ok((rest, ParseResult::Partial("binary expression", "symbol(s)")))
                }
                Err(e) => return Err(e),
            };

            rest = tmp_rest;
            lhs = Expr::BinaryExpr(Box::new(lhs), op, Box::new(rhs));
        }

        Ok((rest, ParseResult::Complete(lhs)))
    }
}

fn binary_op(input: Span) -> IResult<BinaryOp> {
    let (rest, m) = alt((
        char('+'),
        char('/'),
        char('*'),
        char('%'),
        char('^'),
        char('-'),
    ))(input)?;
    Ok((
        rest,
        match m {
            '+' => BinaryOp::Add,
            '/' => BinaryOp::Div,
            '*' => BinaryOp::Mul,
            '%' => BinaryOp::Mod,
            '^' => BinaryOp::Pow,
            '-' => BinaryOp::Sub,
            _ => unreachable!(),
        },
    ))
}

fn expr_unary(input: Span) -> IResult<ParseResult<Expr>> {
    let (rest, (op, expr)) =
        match pair(unary_op, maybe_lpadded(expr(BinaryOp::Mul.precedence())))(input)? {
            (r, (o, ParseResult::Complete(e))) => (r, (o, e)),
            (r, (_, ParseResult::Partial(w, u))) => return Ok((r, ParseResult::Partial(w, u))),
        };

    Ok((
        rest,
        ParseResult::Complete(Expr::UnaryExpr(op, Box::new(expr))),
    ))
}

fn unary_op(input: Span) -> IResult<UnaryOp> {
    let (rest, m) = alt((char('+'), char('-')))(input)?;
    Ok((
        rest,
        match m {
            '+' => UnaryOp::Add,
            '-' => UnaryOp::Sub,
            _ => unreachable!(),
        },
    ))
}

fn expr_vector_selector(input: Span) -> IResult<ParseResult<Expr>> {
    let (rest, selector) = match vector_selector(input)? {
        (r, ParseResult::Complete(s)) => (r, s),
        (r, ParseResult::Partial(w, u)) => return Ok((r, ParseResult::Partial(w, u))),
    };
    Ok((rest, ParseResult::Complete(Expr::VectorSelector(selector))))
}

/// expr_number_literal uses ParseResult to unify the caller side.
fn expr_number_literal(input: Span) -> IResult<ParseResult<Expr>> {
    let (rest, n) = double(input)?;
    Ok((rest, ParseResult::Complete(Expr::NumberLiteral(n))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::result::ParseError;

    #[test]
    fn test_valid_expressions() -> std::result::Result<(), nom::Err<ParseError<'static>>> {
        #[rustfmt::skip]
        let tests = [
            "foo{}",
            "-foo{}",
            "- foo{}",
            "+foo{}",
            "+  foo{}",
        ];

        for input in &tests {
            match expr(Precedence::MIN)(Span::new(input))? {
                (_, ParseResult::Complete(_)) => (),
                (_, ParseResult::Partial(w, u)) => {
                    panic!(
                        "valid expression {} couldn't be parsed: {} {}",
                        *input, w, u
                    );
                }
            };
        }
        Ok(())
    }

    #[test]
    fn test_valid_expressions_ex() -> std::result::Result<(), nom::Err<ParseError<'static>>> {
        use Expr::*;

        let tests = [
            (
                "-1 + 2",
                BinaryExpr(
                    Box::new(NumberLiteral(-1.0)),
                    BinaryOp::Add,
                    Box::new(NumberLiteral(2.0)),
                ),
            ),
            (
                "-1 * 2",
                BinaryExpr(
                    Box::new(NumberLiteral(-1.0)),
                    BinaryOp::Mul,
                    Box::new(NumberLiteral(2.0)),
                ),
            ),
            (
                "-1 ^ 2",
                BinaryExpr(
                    Box::new(NumberLiteral(-1.0)),
                    BinaryOp::Pow,
                    Box::new(NumberLiteral(2.0)),
                ),
            ),
            (
                "-1 ^ 2 * 3",
                BinaryExpr(
                    Box::new(BinaryExpr(
                        Box::new(NumberLiteral(-1.0)),
                        BinaryOp::Pow,
                        Box::new(NumberLiteral(2.0)),
                    )),
                    BinaryOp::Mul,
                    Box::new(NumberLiteral(3.0)),
                ),
            ),
            (
                "1 - -2",
                BinaryExpr(
                    Box::new(NumberLiteral(1.0)),
                    BinaryOp::Sub,
                    Box::new(NumberLiteral(-2.0)),
                ),
            ),
            (
                "-1---2",
                BinaryExpr(
                    Box::new(NumberLiteral(-1.0)),
                    BinaryOp::Sub,
                    Box::new(UnaryExpr(UnaryOp::Sub, Box::new(NumberLiteral(-2.0)))),
                ),
            ),
            (
                "-1---2+3",
                BinaryExpr(
                    Box::new(BinaryExpr(
                        Box::new(NumberLiteral(-1.0)),
                        BinaryOp::Sub,
                        Box::new(UnaryExpr(UnaryOp::Sub, Box::new(NumberLiteral(-2.0)))),
                    )),
                    BinaryOp::Add,
                    Box::new(NumberLiteral(3.0)),
                ),
            ),
            // TODO: "-1---2*3-4",
            (
                "1 + -4*2^3 -5",
                BinaryExpr(
                    Box::new(BinaryExpr(
                        Box::new(NumberLiteral(1.0)),
                        BinaryOp::Add,
                        Box::new(BinaryExpr(
                            Box::new(NumberLiteral(-4.0)),
                            BinaryOp::Mul,
                            Box::new(BinaryExpr(
                                Box::new(NumberLiteral(2.0)),
                                BinaryOp::Pow,
                                Box::new(NumberLiteral(3.0)),
                            )),
                        )),
                    )),
                    BinaryOp::Sub,
                    Box::new(NumberLiteral(5.0)),
                ),
            ),
        ];

        for (input, expected_expr) in &tests {
            let actual_expr = match expr(Precedence::MIN)(Span::new(input))? {
                (r, ParseResult::Partial(w, u)) => {
                    panic!("Unexpected partial parse result: {}, {}, {}", r, w, u)
                }
                (_, ParseResult::Complete(e)) => e,
            };
            assert_eq!(expected_expr, &actual_expr, "while parsing {}", input);
        }
        Ok(())
    }

    #[test]
    fn test_binary_expr_operator_precedence(
    ) -> std::result::Result<(), nom::Err<ParseError<'static>>> {
        #[rustfmt::skip]
        let tests = [
            ("42 + bar", vec![BinaryOp::Add]),
            ("42.42 + bar", vec![BinaryOp::Add]),
            ("42.42 + bar % 9000", vec![BinaryOp::Mod, BinaryOp::Add]),
            ("-42.42 + -bar % 9000", vec![BinaryOp::Mod, BinaryOp::Add]),
            ("foo + bar", vec![BinaryOp::Add]),
            ("foo + bar - baz", vec![BinaryOp::Add, BinaryOp::Sub]),
            ("foo + bar * baz", vec![BinaryOp::Mul, BinaryOp::Add]),
            ("foo * bar + baz", vec![BinaryOp::Mul, BinaryOp::Add]),
            ("foo * bar ^ baz", vec![BinaryOp::Pow, BinaryOp::Mul]),
            ("foo * bar ^ baz - qux / abc", vec![BinaryOp::Pow, BinaryOp::Mul, BinaryOp::Div, BinaryOp::Sub]),
        ];

        fn extract_operators(expr: Box<Expr>) -> Vec<BinaryOp> {
            match *expr {
                Expr::BinaryExpr(lhs, op, rhs) => extract_operators(lhs)
                    .into_iter()
                    .chain(extract_operators(rhs).into_iter())
                    .chain(vec![op].into_iter())
                    .collect(),
                Expr::UnaryExpr(_, expr) => extract_operators(expr),
                _ => vec![],
            }
        }

        for (input, expected_ops) in &tests {
            let ex = match expr(Precedence::MIN)(Span::new(input))? {
                (r, ParseResult::Partial(w, u)) => {
                    panic!("Unexpected partial parse result: {}, {}, {}", r, w, u)
                }
                (_, ParseResult::Complete(e)) => e,
            };
            assert_eq!(expected_ops, &extract_operators(Box::new(ex)));
        }
        Ok(())
    }
}
