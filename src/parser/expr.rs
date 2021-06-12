use std::convert::TryFrom;

use nom::{
    branch::alt, bytes::complete::tag_no_case, character::complete::char, number::complete::double,
    sequence::pair,
};

use super::ast::{
    BinaryOp, Expr, GroupModifier, Precedence, UnaryOp, VectorMatching, VectorMatchingKind,
};
use super::common::{label_identifier, maybe_lpadded, separated_list};
use super::result::{IResult, ParseError, Span};
use super::vector::vector_selector;

pub fn expr<'a>(min_prec: Option<Precedence>) -> impl FnMut(Span<'a>) -> IResult<Expr> {
    move |input: Span| {
        let (mut rest, mut lhs) = alt((
            // expr_aggregate
            // expr_function_call
            // expr_paren
            expr_number_literal,
            expr_unary,
            expr_vector_selector,
            // expr_matrix_selector  <-- consider merging with vector_selector
            // expr_offset           <-- consider merging with vector_selector
        ))(input)?;

        // The rest is dealing with the left-recursive grammar.
        // E.g.  expr = unary_expr | vector_selector | binary_expr ...
        // where binary_expr = expr <OP> expr

        while *rest != "" {
            let (tmp_rest, op) = match maybe_lpadded(binary_op)(rest) {
                Ok((r, o)) => (r, o),
                Err(_) => {
                    return Err(nom::Err::Failure(ParseError::partial(
                        "binary expression",
                        "symbol(s)",
                        rest,
                    )))
                }
            };

            if op.precedence() <= min_prec.unwrap_or(Precedence::MIN) {
                break;
            }
            rest = tmp_rest;

            // So, it IS a binary expression since we have the `lhs` and the `op`.

            // TODO: Validate: lhs is an instant vector selector or scalar.

            let (tmp_rest, bool_modifier) = match maybe_lpadded(bool_modifier)(rest) {
                Ok((r, _)) => (r, true),
                Err(nom::Err::Error(_)) => (rest, false),
                Err(e) => return Err(e),
            };
            rest = tmp_rest;

            // TODO: Validate: bool_modifier can only be used with a comparison binary op.

            let (tmp_rest, vector_matching) = match maybe_lpadded(vector_matching)(rest) {
                Ok((r, vm)) => (r, Some(vm)),
                Err(nom::Err::Error(_)) => (rest, None),
                Err(e) => return Err(e),
            };
            rest = tmp_rest;

            let (tmp_rest, group_modifier) = match maybe_lpadded(group_modifier)(rest) {
                Ok((r, gm)) => (r, Some(gm)),
                Err(nom::Err::Error(_)) => (rest, None),
                Err(e) => return Err(e),
            };
            rest = tmp_rest;

            // TODO: Validate:
            //   - if group_modifier is present, vector_matching must be present
            //   - if group_modifier is present, op must not be AND, OR, or UNLESS
            //   - if vector_matching is 'on', vector_matching & group_modifier intersection
            //     must result in empty set.

            let (tmp_rest, rhs) = match maybe_lpadded(expr(Some(op.precedence())))(rest) {
                Ok((r, e)) => (r, e),
                Err(nom::Err::Error(_)) => {
                    return Err(nom::Err::Failure(ParseError::partial(
                        "binary expression",
                        "symbol(s)",
                        rest,
                    )))
                }
                Err(e) => return Err(e),
            };

            // TODO: Validate:
            //   - rhs is an instant vector selector or scalar
            //   - if (lhs, op, rhs) is (scalar, comparison, scalar), the bool_modifier must be present.
            //   - if vector_matching is present, lhs and rhs must be instant vectors.

            rest = tmp_rest;
            lhs = Expr::BinaryExpr(Box::new(lhs), op, Box::new(rhs));
        }

        Ok((rest, lhs))
    }
}

fn binary_op(input: Span) -> IResult<BinaryOp> {
    let (rest, op) = alt((
        tag_no_case("+"),
        tag_no_case("/"),
        tag_no_case("*"),
        tag_no_case("%"),
        tag_no_case("^"),
        tag_no_case("-"),
        tag_no_case("=="),
        tag_no_case(">="),
        tag_no_case(">"),
        tag_no_case("<"),
        tag_no_case("<="),
        tag_no_case("!="),
        tag_no_case("and"),
        tag_no_case("unless"),
        tag_no_case("or"),
    ))(input)?;
    Ok((rest, BinaryOp::try_from(*op).unwrap()))
}

fn bool_modifier(input: Span) -> IResult<()> {
    let (rest, _) = tag_no_case("bool")(input)?;
    Ok((rest, ()))
}

fn vector_matching(input: Span) -> IResult<VectorMatching> {
    let (rest, kind) = alt((tag_no_case("on"), tag_no_case("ignoring")))(input)?;

    let kind = VectorMatchingKind::try_from(*kind).unwrap();
    let (rest, labels) = maybe_lpadded(grouping_labels)(rest)?;

    Ok((rest, VectorMatching::new(kind, labels)))
}

fn group_modifier(input: Span) -> IResult<GroupModifier> {
    let (rest, modifier) = alt((tag_no_case("group_left"), tag_no_case("group_right")))(input)?;

    let (rest, labels) = match maybe_lpadded(grouping_labels)(rest) {
        Ok((r, ls)) => (r, ls),
        Err(nom::Err::Error(_)) => (rest, vec![]),
        Err(e) => return Err(e),
    };

    if modifier.to_lowercase() == "group_left" {
        Ok((rest, GroupModifier::Left(labels)))
    } else {
        Ok((rest, GroupModifier::Right(labels)))
    }
}

fn grouping_labels(input: Span) -> IResult<Vec<String>> {
    separated_list(
        '(',
        ')',
        ',',
        label_identifier,
        "grouping labels clause",
        r#"label or ")""#,
    )(input)
}

fn expr_unary(input: Span) -> IResult<Expr> {
    let (rest, (op, expr)) = pair(
        unary_op,
        maybe_lpadded(expr(Some(BinaryOp::Mul.precedence()))),
    )(input)?;

    Ok((rest, Expr::UnaryExpr(op, Box::new(expr))))
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

fn expr_vector_selector(input: Span) -> IResult<Expr> {
    let (rest, vs) = vector_selector(input)?;
    Ok((rest, Expr::VectorSelector(vs)))
}

fn expr_number_literal(input: Span) -> IResult<Expr> {
    let (rest, n) = double(input)?;
    Ok((rest, Expr::NumberLiteral(n)))
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
            "foo and bar",
            "foo unless bar",
            "foo or bar",
        ];

        for input in &tests {
            expr(None)(Span::new(input))?;
        }
        Ok(())
    }

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
            let (_, actual_expr) = expr(None)(Span::new(input))?;
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
            let (_, ex) = expr(None)(Span::new(input))?;
            assert_eq!(expected_ops, &extract_operators(Box::new(ex)));
        }
        Ok(())
    }

    #[test]
    fn test_binary_expr_bool_modifier() -> std::result::Result<(), nom::Err<ParseError<'static>>> {
        // bool modifier can be used only with comparison binary op.
        // but, between any supported data types:
        //   - scalar & scalar
        //   - scalar & vector
        //   - vector & vector
        #[rustfmt::skip]
         let tests = [
             ("1 >  bool 1", "foo"),
             ("1 == bool 1", "foo"),
             ("1 < bool 2 - 1 * 2", "foo"),
             ("foo != bool 1", "foo"),
             ("foo != bool bar", "foo"),
         ];

        for (input, _expected_ops) in &tests {
            let (_, ex) = expr(None)(Span::new(input))?;
            // TODO: add assertion
            println!("{:?}", ex);
        }
        Ok(())
    }

    #[test]
    fn test_binary_expr_vector_matching() -> std::result::Result<(), nom::Err<ParseError<'static>>>
    {
        #[rustfmt::skip]
         let tests = [
             ("foo * on() bar", "foo"),
             ("foo % ignoring() bar", "foo"),
             ("foo + on(abc) bar", "foo"),
             ("foo != on(abc,def) bar", "foo"),
             ("foo > on(abc,def,) bar", "foo"),
             ("foo - on(abc) bar / on(qux, lol) baz", "foo"),
         ];

        for (input, _expected_ops) in &tests {
            let (_, ex) = expr(None)(Span::new(input))?;
            // TODO: add assertion
            println!("{:?}", ex);
        }
        Ok(())
    }

    #[test]
    fn test_group_modifier() -> std::result::Result<(), nom::Err<ParseError<'static>>> {
        #[rustfmt::skip]
         let tests = [
             ("foo * on(test) group_left bar", "foo"),
             ("foo * on(test,blub) group_left() bar", "foo"),
             ("foo + ignoring(abc) group_right (qux) bar", "foo"),
             ("foo + ignoring(abc) group_right(def,qux,) bar", "foo"),
         ];

        for (input, _expected_ops) in &tests {
            let (_, ex) = expr(None)(Span::new(input))?;
            // TODO: add assertion
            println!("{:?}", ex);
        }
        Ok(())
    }
}
