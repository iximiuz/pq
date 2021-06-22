use std::convert::TryFrom;

use nom::{
    branch::alt,
    bytes::complete::tag_no_case,
    character::complete::char,
    sequence::{pair, terminated},
};

use super::ast::*;
use super::common::{label_identifier, maybe_lpadded, separated_list};
use super::number::{expr_number_literal, number_literal};
use super::result::{IResult, ParseError, Span};
use super::string::string_literal;
use super::vector::expr_vector_selector;
use crate::model::types::LabelName;

pub fn expr<'a>(min_prec: Option<Precedence>) -> impl FnMut(Span<'a>) -> IResult<Expr> {
    move |input: Span| {
        let (mut rest, mut lhs) = alt((
            // Order matters here!
            expr_aggregate,
            expr_paren,
            expr_number_literal,
            expr_unary,
            expr_vector_selector,
            expr_function_call,
            // TODO: expr_offset           <-- consider merging with vector_selector
        ))(input)?;

        // The rest is dealing with the left-recursive grammar.
        // E.g.  expr = unary_expr | vector_selector | binary_expr ...
        // where binary_expr = expr <OP> expr

        while *rest != "" && !(*rest).starts_with(")") {
            let (tmp_rest, op) = match maybe_lpadded(binary_op)(rest) {
                Ok((r, o)) => (r, o),
                Err(_) => {
                    return Err(nom::Err::Failure(ParseError::partial(
                        "binary expression",
                        "binary operator",
                        rest,
                    )))
                }
            };

            if op.precedence() <= min_prec.unwrap_or(Precedence::MIN) {
                break;
            }
            rest = tmp_rest;

            // So, it IS a binary expression since we have the `lhs` and the `op`.

            // TODO: validate - lhs is an instant vector selector or scalar.

            let (tmp_rest, bool_modifier) = match maybe_lpadded(bool_modifier)(rest) {
                Ok((r, _)) => (r, true),
                Err(nom::Err::Error(_)) => (rest, false),
                Err(e) => return Err(e),
            };
            rest = tmp_rest;

            // TODO: validate - bool_modifier can only be used with a comparison binary op.

            let (tmp_rest, label_matching) = match maybe_lpadded(label_matching)(rest) {
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

            // TODO: validate
            //   - if group_modifier is present, label_matching must be present
            //   - if group_modifier is present, op must not be AND, OR, or UNLESS
            //   - if label_matching_kind is 'on', label_matching & group_modifier
            //     intersection must be an empty set.

            let (tmp_rest, rhs) = match maybe_lpadded(expr(Some(op.precedence())))(rest) {
                Ok((r, e)) => (r, e),
                Err(nom::Err::Error(_)) => {
                    return Err(nom::Err::Failure(ParseError::partial(
                        "binary expression",
                        "right-hand expression",
                        rest,
                    )))
                }
                Err(e) => return Err(e),
            };

            // TODO: validate
            //   - rhs is an instant vector selector or scalar
            //   - if (lhs, op, rhs) is (scalar, comparison, scalar), the bool_modifier must be present.
            //   - if label_matching is present, lhs and rhs must be instant vectors.

            rest = tmp_rest;
            lhs = Expr::BinaryExpr(BinaryExpr::new_ex(
                lhs,
                op,
                rhs,
                bool_modifier,
                label_matching,
                group_modifier,
            ));
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
        tag_no_case("<="),
        tag_no_case("<"),
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

fn label_matching(input: Span) -> IResult<LabelMatching> {
    let (rest, kind) = alt((tag_no_case("on"), tag_no_case("ignoring")))(input)?;

    let (rest, labels) = maybe_lpadded(grouping_labels)(rest)?;

    if kind.to_lowercase() == "on" {
        Ok((rest, LabelMatching::On(labels.into_iter().collect())))
    } else {
        Ok((rest, LabelMatching::Ignoring(labels.into_iter().collect())))
    }
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

fn expr_aggregate(input: Span) -> IResult<Expr> {
    // First, parse the operator type.
    let (rest, op) = aggregate_op(input)?;

    // Then maybe parse a modifier.
    let (rest, modifier) = match maybe_lpadded(aggregate_modifier)(rest) {
        Ok((rest, modifier)) => (rest, Some(modifier)),
        Err(nom::Err::Error(_)) => (rest, None),
        Err(e) => return Err(e),
    };

    // Then mandatory operator's body (...).
    let (rest, _) = match maybe_lpadded(char('('))(rest) {
        Ok((rest, _)) => (rest, '_'),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "aggregate expression",
                "(",
                rest,
            )))
        }
        Err(e) => return Err(e),
    };

    // Some operators have a mandatory argument, try parsing it.
    use AggregateOp::*;
    let (rest, argument) = match op {
        CountValues => match maybe_lpadded(string_literal)(rest) {
            Ok((rest, s)) => (rest, Some(AggregateArgument::String(s))),
            Err(nom::Err::Error(_)) => {
                return Err(nom::Err::Failure(ParseError::partial(
                    "count_values operator",
                    "string literal",
                    rest,
                )))
            }
            Err(e) => return Err(e),
        },
        Quantile | TopK | BottomK => match maybe_lpadded(number_literal)(rest) {
            Ok((rest, n)) => (rest, Some(AggregateArgument::Number(n))),
            Err(nom::Err::Error(_)) => {
                return Err(nom::Err::Failure(ParseError::partial(
                    "quantile, topk, or bottomk operator",
                    "number literal",
                    rest,
                )))
            }
            Err(e) => return Err(e),
        },
        _ => (rest, None),
    };

    // If argument is there, it should be followed by a comma.
    let (rest, _) = match argument {
        Some(_) => match maybe_lpadded(char(','))(rest) {
            Ok((rest, _)) => (rest, '_'),
            Err(nom::Err::Error(_)) => {
                return Err(nom::Err::Failure(ParseError::partial(
                    "count_values, quantile, topk, or bottomk operator",
                    ",",
                    rest,
                )))
            }
            Err(e) => return Err(e),
        },
        None => (rest, '_'),
    };

    // Finally, parse the inner expression.
    let (rest, inner_expr) = match maybe_lpadded(expr(None))(rest) {
        Ok((rest, ex)) => (rest, ex),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "aggregate operator",
                "valid expression",
                rest,
            )))
        }
        Err(e) => return Err(e),
    };

    // TODO: validate that inner_expr evaluates to instant vector.

    // Finalizing operator's body.
    let (rest, _) = match maybe_lpadded(char(')'))(rest) {
        Ok((rest, _)) => (rest, '_'),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "aggregate expression",
                ")",
                rest,
            )))
        }
        Err(e) => return Err(e),
    };

    // If modifier wasn't found in the prefix, agg op may have a trailing modifier.
    let (rest, modifier) = match modifier {
        Some(m) => (rest, Some(m)),
        None => match maybe_lpadded(aggregate_modifier)(rest) {
            Ok((rest, modifier)) => (rest, Some(modifier)),
            Err(nom::Err::Error(_)) => (rest, None),
            Err(e) => return Err(e),
        },
    };

    Ok((
        rest,
        Expr::AggregateExpr(AggregateExpr::new(op, inner_expr, modifier, argument)),
    ))
}

fn aggregate_op(input: Span) -> IResult<AggregateOp> {
    let (rest, op) = alt((
        tag_no_case("avg"),
        tag_no_case("bottomk"),
        tag_no_case("count"),
        tag_no_case("count_values"),
        tag_no_case("group"),
        tag_no_case("max"),
        tag_no_case("min"),
        tag_no_case("quantile"),
        tag_no_case("stddev"),
        tag_no_case("stdvar"),
        tag_no_case("sum"),
        tag_no_case("topk"),
    ))(input)?;
    Ok((rest, AggregateOp::try_from(*op).unwrap()))
}

fn aggregate_modifier(input: Span) -> IResult<AggregateModifier> {
    let (rest, modifier) = alt((tag_no_case("by"), tag_no_case("without")))(input)?;

    let (rest, labels) = match maybe_lpadded(grouping_labels)(rest) {
        Ok((r, ls)) => (r, ls),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "aggregation",
                "label list",
                rest,
            )))
        }
        Err(e) => return Err(e),
    };

    if modifier.to_lowercase() == "by" {
        Ok((rest, AggregateModifier::By(labels.into_iter().collect())))
    } else {
        Ok((
            rest,
            AggregateModifier::Without(labels.into_iter().collect()),
        ))
    }
}

fn grouping_labels(input: Span) -> IResult<Vec<LabelName>> {
    separated_list(
        '(',
        ')',
        ',',
        label_identifier,
        "grouping labels clause",
        r#"label or ")""#,
    )(input)
}

/// Parse parenthesized expression like '(' <expr> ')'.
fn expr_paren(input: Span) -> IResult<Expr> {
    let (rest, _) = char('(')(input)?;

    let (rest, inner_expr) = match maybe_lpadded(expr(None))(rest) {
        Ok((rest, ex)) => (rest, ex),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "parentheses",
                "valid expression",
                rest,
            )))
        }
        Err(e) => return Err(e),
    };

    let (rest, _) = match maybe_lpadded(char(')'))(rest) {
        Ok((rest, _)) => (rest, '_'),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "parentheses",
                ")",
                rest,
            )))
        }
        Err(e) => return Err(e),
    };

    Ok((rest, Expr::Parentheses(Box::new(inner_expr))))
}

/// Parse unary expression like '-' <expr> or '+' <expr>.
fn expr_unary(input: Span) -> IResult<Expr> {
    let (rest, (op, expr)) = pair(
        unary_op,
        maybe_lpadded(expr(Some(BinaryOp::Mul.precedence()))),
    )(input)?;

    // TODO: validate - expression is scalar or instant vector.

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

fn expr_function_call(input: Span) -> IResult<Expr> {
    let (rest, func_name) = terminated(function_name, maybe_lpadded(char('(')))(input)?;

    // function_call() should never return nom::Err::Error.
    let (rest, func_call) = function_call(func_name, rest)?;

    let (rest, _) = match maybe_lpadded(char(')'))(rest) {
        Ok((rest, c)) => (rest, c),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "function call",
                ")",
                rest,
            )))
        }
        Err(e) => return Err(e),
    };

    Ok((rest, Expr::FunctionCall(func_call)))
}

fn function_name(input: Span) -> IResult<FunctionName> {
    let (rest, name) = alt((
        tag_no_case("clamp"),
        tag_no_case("clamp_max"),
        tag_no_case("clamp_min"),
        tag_no_case("count_over_time"),
        tag_no_case("last_over_time"),
        tag_no_case("max_over_time"),
        tag_no_case("min_over_time"),
        tag_no_case("sum_over_time"),
    ))(input)?;
    Ok((rest, FunctionName::try_from(*name).unwrap()))
}

/// It should never return nom::Err::Error. Only success or total failure.
fn function_call(func_name: FunctionName, input: Span) -> IResult<FunctionCall> {
    use FunctionName::*;

    let arg_parsers: Vec<fn(Span) -> IResult<FunctionArg>> = match func_name {
        CountOverTime | LastOverTime | MaxOverTime | MinOverTime | SumOverTime => {
            vec![func_arg_instant_vector]
        }
        Clamp => vec![func_arg_instant_vector, func_arg_number, func_arg_number],
        ClampMax | ClampMin => vec![func_arg_instant_vector, func_arg_number],
        Vector => vec![func_arg_number],
    };

    let (rest, args) = func_args(&arg_parsers, input)?;

    Ok((rest, FunctionCall::new(func_name, args)))
}

/// It should never return nom::Err::Error. Only success or total failure.
fn func_args<'a, F>(arg_parsers: &[F], input: Span<'a>) -> IResult<'a, Vec<FunctionArg>>
where
    F: Fn(Span<'a>) -> IResult<FunctionArg>,
{
    let mut args = Vec::new();
    let mut rest = input;

    let mut iter = arg_parsers.iter().peekable();
    loop {
        let parse = match iter.next() {
            Some(parse) => parse,
            None => break,
        };

        let (tmp_rest, arg) = parse(rest)?;
        args.push(arg);
        rest = tmp_rest;
    }

    // TODO: check for trailing comma explicitly.

    Ok((rest, args))
}

/// It should never return nom::Err::Error. Only success or total failure.
fn func_arg_number(input: Span) -> IResult<FunctionArg> {
    match number_literal(input) {
        Ok((rest, n)) => Ok((rest, FunctionArg::Number(n))),
        Err(nom::Err::Failure(e)) => Err(nom::Err::Failure(e)),
        _ => Err(nom::Err::Failure(ParseError::partial(
            "function call",
            "number literal",
            input,
        ))),
    }
}

/// It should never return nom::Err::Error. Only success or total failure.
fn func_arg_string(input: Span) -> IResult<FunctionArg> {
    match string_literal(input) {
        Ok((rest, s)) => Ok((rest, FunctionArg::String(s))),
        Err(nom::Err::Failure(e)) => Err(nom::Err::Failure(e)),
        _ => Err(nom::Err::Failure(ParseError::partial(
            "function call",
            "string literal",
            input,
        ))),
    }
}

/// It should never return nom::Err::Error. Only success or total failure.
fn func_arg_instant_vector(input: Span) -> IResult<FunctionArg> {
    match expr(None)(input) {
        // TODO: check that expr evaluates to an instant vector.
        Ok((rest, expr)) => Ok((rest, FunctionArg::Expr(Box::new(expr)))),
        Err(nom::Err::Failure(e)) => Err(nom::Err::Failure(e)),
        _ => Err(nom::Err::Failure(ParseError::partial(
            "function call",
            "instant vector",
            input,
        ))),
    }
}

/// It should never return nom::Err::Error. Only success or total failure.
fn func_arg_range_vector(input: Span) -> IResult<FunctionArg> {
    match expr(None)(input) {
        // TODO: check that expr evaluates to a range vector.
        Ok((rest, expr)) => Ok((rest, FunctionArg::Expr(Box::new(expr)))),
        Err(nom::Err::Failure(e)) => Err(nom::Err::Failure(e)),
        _ => Err(nom::Err::Failure(ParseError::partial(
            "function call",
            "range vector",
            input,
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::result::ParseError;

    #[test]
    fn test_valid_expressions() -> std::result::Result<(), String> {
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
            "sum(foo)",
            "sum(foo) by(job)",
            "bar{} + sum(foo) by(job)",
            "avg(foo) without(job,instanse)",
            "sum by(job) (foo)",
            "avg without(job,instanse) (foo)",
            "124 % avg without(job,instanse) (foo)",
            "quantile(0.95, foo)",
            "topk(3, foo)",
            "bottomk(1.0, foo)",
            "(foo)",
            "(1 + 2) * 3",
        ];

        for input in &tests {
            expr(None)(Span::new(input))
                .map_err(|e| format!("Got {:?} while parsing {}", e, input))?;
        }
        Ok(())
    }

    #[test]
    fn test_valid_expressions_ex() -> std::result::Result<(), nom::Err<ParseError<'static>>> {
        use super::BinaryExpr as BinaryExprInner;
        use Expr::*;

        let tests = [
            (
                "-1 + 2",
                BinaryExpr(BinaryExprInner::new(
                    NumberLiteral(-1.0),
                    BinaryOp::Add,
                    NumberLiteral(2.0),
                )),
            ),
            (
                "-1 * 2",
                BinaryExpr(BinaryExprInner::new(
                    NumberLiteral(-1.0),
                    BinaryOp::Mul,
                    NumberLiteral(2.0),
                )),
            ),
            (
                "-1 ^ 2",
                BinaryExpr(BinaryExprInner::new(
                    NumberLiteral(-1.0),
                    BinaryOp::Pow,
                    NumberLiteral(2.0),
                )),
            ),
            (
                "-1 ^ 2 * 3",
                BinaryExpr(BinaryExprInner::new(
                    BinaryExpr(BinaryExprInner::new(
                        NumberLiteral(-1.0),
                        BinaryOp::Pow,
                        NumberLiteral(2.0),
                    )),
                    BinaryOp::Mul,
                    NumberLiteral(3.0),
                )),
            ),
            (
                "1 - -2",
                BinaryExpr(BinaryExprInner::new(
                    NumberLiteral(1.0),
                    BinaryOp::Sub,
                    NumberLiteral(-2.0),
                )),
            ),
            (
                "-1---2",
                BinaryExpr(BinaryExprInner::new(
                    NumberLiteral(-1.0),
                    BinaryOp::Sub,
                    UnaryExpr(UnaryOp::Sub, Box::new(NumberLiteral(-2.0))),
                )),
            ),
            (
                "-1---2+3",
                BinaryExpr(BinaryExprInner::new(
                    BinaryExpr(BinaryExprInner::new(
                        NumberLiteral(-1.0),
                        BinaryOp::Sub,
                        UnaryExpr(UnaryOp::Sub, Box::new(NumberLiteral(-2.0))),
                    )),
                    BinaryOp::Add,
                    NumberLiteral(3.0),
                )),
            ),
            // TODO: "-1---2*3-4",
            (
                "1 + -4*2^3 -5",
                BinaryExpr(BinaryExprInner::new(
                    BinaryExpr(BinaryExprInner::new(
                        NumberLiteral(1.0),
                        BinaryOp::Add,
                        BinaryExpr(BinaryExprInner::new(
                            NumberLiteral(-4.0),
                            BinaryOp::Mul,
                            BinaryExpr(BinaryExprInner::new(
                                NumberLiteral(2.0),
                                BinaryOp::Pow,
                                NumberLiteral(3.0),
                            )),
                        )),
                    )),
                    BinaryOp::Sub,
                    NumberLiteral(5.0),
                )),
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

        fn extract_operators(expr: &Expr) -> Vec<BinaryOp> {
            match expr {
                Expr::BinaryExpr(e) => extract_operators(e.lhs())
                    .into_iter()
                    .chain(extract_operators(e.rhs()).into_iter())
                    .chain(vec![e.op()].into_iter())
                    .collect(),
                Expr::UnaryExpr(_, e) => extract_operators(e.as_ref()),
                _ => vec![],
            }
        }

        for (input, expected_ops) in &tests {
            let (_, ex) = expr(None)(Span::new(input))?;
            assert_eq!(expected_ops, &extract_operators(&ex));
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
    fn test_binary_expr_label_matching() -> std::result::Result<(), nom::Err<ParseError<'static>>> {
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
