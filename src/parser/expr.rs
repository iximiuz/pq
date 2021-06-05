use nom::{branch::alt, character::complete::char, sequence::pair};

use super::ast::{BinaryOp, Expr, UnaryOp};
use super::common::maybe_lpadded;
use super::result::{IResult, ParseResult, Span};
use super::vector::vector_selector;

pub fn expr(input: Span) -> IResult<ParseResult<Expr>> {
    let (rest, lhs) = match alt((expr_unary, expr_vector_selector))(input)? {
        (r, ParseResult::Complete(e)) => (r, e),
        (r, ParseResult::Partial(w, u)) => return Ok((r, ParseResult::Partial(w, u))),
    };

    if *rest == "" {
        // Non-compound expression, we are done here.
        return Ok((rest, ParseResult::Complete(lhs)));
    }

    // The rest is dealing with the left-recursive grammar.
    // E.g.  expr = unary_expr | binary_expr | vector_selector | ...
    // where binary_expr = expr <OP> expr

    let (rest, op) = match maybe_lpadded(binary_op)(rest) {
        Ok((r, o)) => (r, o),
        Err(nom::Err::Error(_)) => {
            return Ok((rest, ParseResult::Partial("expression", "symbol(s)")))
        }
        Err(e) => return Err(e),
    };

    let (rest, rhs) = match maybe_lpadded(expr)(rest) {
        Ok((r, ParseResult::Complete(e))) => (r, e),
        Ok((r, ParseResult::Partial(w, u))) => return Ok((r, ParseResult::Partial(w, u))),
        Err(nom::Err::Error(_)) => {
            return Ok((rest, ParseResult::Partial("expression", "symbol(s)")))
        }
        Err(e) => return Err(e),
    };

    if *rest != "" {
        return Ok((rest, ParseResult::Partial("expression", "symbol(s)")));
    }

    // The `rhs` can itself be a binary expr. Its operator may though may have a lower
    // precendence than `op` from above. That is, we may need to regroup expressions
    // here to fix the precendence in the resulting compound expression.
    let (lhs, op, rhs) = match rhs {
        Expr::BinaryExpr(l, o, r) if o.precendence() >= op.precendence() => {
            // Nope, false alarm. Restore `rhs` as it used to be.
            (lhs, op, Expr::BinaryExpr(l, o, r))
        }
        Expr::BinaryExpr(l, o, r) => {
            // Yep! Fixing the precendence by reorganizing expressions.
            (Expr::BinaryExpr(Box::new(lhs), op, l), o, *r)
        }
        // The `rhs` is not even a binary expr.
        _ => (lhs, op, rhs),
    };

    return Ok((
        rest,
        ParseResult::Complete(Expr::BinaryExpr(Box::new(lhs), op, Box::new(rhs))),
    ));
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
    let (rest, (op, expr)) = match pair(unary_op, maybe_lpadded(expr))(input)? {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::result::ParseError;

    #[test]
    fn test_expr_valid() -> std::result::Result<(), nom::Err<ParseError<'static>>> {
        #[rustfmt::skip]
        let tests = [
            "foo{}",
            "-foo{}",
            "- foo{}",
            "+foo{}",
            "+  foo{}",
        ];

        for input in &tests {
            let ex = expr(Span::new(input))?;
            println!("{:#?}", ex);
            // TODO: add assertions
        }
        Ok(())
    }

    #[test]
    fn test_binary_expr_operator_precedence(
    ) -> std::result::Result<(), nom::Err<ParseError<'static>>> {
        #[rustfmt::skip]
        let tests = [
            ("foo + bar", vec![BinaryOp::Add]),
            ("foo + bar - baz", vec![BinaryOp::Sub, BinaryOp::Add]),
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
                _ => vec![],
            }
        }

        for (input, expected_ops) in &tests {
            let ex = match expr(Span::new(input))? {
                (r, ParseResult::Partial(w, u)) => {
                    panic!("Unexpected partial parse result: {}, {}, {}", r, w, u)
                }
                (_, ParseResult::Complete(e)) => e,
            };
            println!("{:#?}", ex);
            assert_eq!(expected_ops, &extract_operators(Box::new(ex)));
        }
        Ok(())
    }
}
