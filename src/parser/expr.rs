use nom::{branch::alt, character::complete::char, sequence::pair};

use super::ast::{Expr, UnaryOp};
use super::common::maybe_lpadded;
use super::result::{IResult, ParseResult, Span};
use super::vector::vector_selector;

pub fn expr(input: Span) -> IResult<ParseResult<Expr>> {
    //   unary_expr
    // | vector_selector

    alt((expr_unary, expr_vector_selector))(input)
}

fn expr_unary(input: Span) -> IResult<ParseResult<Expr>> {
    let (rest, (op, expr)) = match pair(unary_op, maybe_lpadded(expr))(input)? {
        (r, (o, ParseResult::Complete(e))) => (r, (o, e)),
        (r, (_, ParseResult::Partial(u, w))) => return Ok((r, ParseResult::Partial(u, w))),
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
        (r, ParseResult::Partial(u, w)) => return Ok((r, ParseResult::Partial(u, w))),
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
}
