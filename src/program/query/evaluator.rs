use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use super::aggregate::AggregateEvaluator;
use super::binary::create_binary_evaluator;
use super::function::{create_func_evaluator, FuncCallArg};
use super::identity::IdentityEvaluator;
use super::parser::ast::*;
use super::sample::SampleReader;
use super::unary::UnaryEvaluator;
use super::value::{QueryValue, QueryValueIter, QueryValueKind};
use super::vector::VectorSelectorEvaluator;
use crate::error::Result;
use crate::model::Timestamp;
use crate::parse::Record;

const DEFAULT_INTERVAL: Duration = Duration::from_millis(1000);

pub struct QueryEvaluator {
    inner: Box<dyn QueryValueIter>,
    drained: bool,
}

impl QueryEvaluator {
    pub fn new(
        query: Expr,
        records: Box<dyn std::iter::Iterator<Item = Result<Record>>>,
        interval: Option<Duration>,
        lookback: Option<Duration>,
        start_at: Option<Timestamp>,
        verbose: bool, // TODO: remove it
    ) -> Result<Self> {
        let interval = interval
            .or_else(|| find_smallest_range(&query))
            .unwrap_or(DEFAULT_INTERVAL);
        assert!(interval.as_secs() + (interval.subsec_nanos() as u64) > 0);

        Ok(Self {
            inner: create_value_iter(
                &Context::new(
                    records,
                    interval,
                    lookback.unwrap_or(interval),
                    start_at,
                    verbose,
                ),
                query,
            ),
            drained: false,
        })
    }
}

impl std::iter::Iterator for QueryEvaluator {
    type Item = Result<QueryValue>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.drained {
            return None;
        }

        // Tiny hack to prevent infinite iteration.
        if self.inner.value_kind() == QueryValueKind::Scalar {
            self.drained = true;
        }

        self.inner.next().map(Ok)
    }
}

struct Context {
    samples: Rc<RefCell<SampleReader>>,
    interval: Duration,
    lookback: Duration,
    start_at: Option<Timestamp>,
}

impl Context {
    fn new(
        records: Box<dyn std::iter::Iterator<Item = Result<Record>>>,
        interval: Duration,
        lookback: Duration,
        start_at: Option<Timestamp>,
        verbose: bool,
    ) -> Self {
        Self {
            samples: Rc::new(RefCell::new(SampleReader::new(records, verbose))),
            interval,
            lookback,
            start_at,
        }
    }
}

fn create_value_iter(ctx: &Context, node: Expr) -> Box<dyn QueryValueIter> {
    match node {
        Expr::Parentheses(expr) => create_value_iter(ctx, *expr),

        Expr::AggregateOperation(expr) => {
            let (op, inner, modifier, parameter) = expr.into_inner();
            Box::new(AggregateEvaluator::new(
                op,
                create_value_iter(ctx, *inner),
                modifier,
                parameter,
            ))
        }

        Expr::UnaryOperation(op, expr) => {
            Box::new(UnaryEvaluator::new(op, create_value_iter(ctx, *expr)))
        }

        Expr::BinaryOperation(expr) => {
            let (op, lhs, rhs, bool_modifier, vector_matching, group_modifier) = expr.into_inner();
            create_binary_evaluator(
                op,
                create_value_iter(ctx, *lhs),
                create_value_iter(ctx, *rhs),
                bool_modifier,
                vector_matching,
                group_modifier,
            )
        }

        Expr::FunctionCall(call) => create_func_evaluator(
            call.function_name(),
            call.args()
                .into_iter()
                .map(|arg| match arg {
                    FunctionCallArg::Number(n) => FuncCallArg::Number(n),
                    FunctionCallArg::String(s) => FuncCallArg::String(s),
                    FunctionCallArg::Expr(expr) => {
                        FuncCallArg::ValueIter(create_value_iter(ctx, *expr))
                    }
                })
                .collect(),
        ),

        // leaf node
        Expr::NumberLiteral(val) => Box::new(IdentityEvaluator::scalar(val)),

        // leaf node
        Expr::VectorSelector(sel) => Box::new(VectorSelectorEvaluator::new(
            SampleReader::cursor(Rc::clone(&ctx.samples)),
            sel,
            ctx.interval,
            ctx.lookback,
            ctx.start_at,
        )),
    }
}

fn find_smallest_range(node: &Expr) -> Option<Duration> {
    match node {
        Expr::Parentheses(expr) => find_smallest_range(expr),

        Expr::AggregateOperation(op) => find_smallest_range(op.expr()),

        Expr::UnaryOperation(_, expr) => find_smallest_range(expr),

        Expr::BinaryOperation(op) => {
            let lhs = find_smallest_range(op.lhs());
            let rhs = find_smallest_range(op.rhs());
            match (lhs, rhs) {
                (None, None) => None,
                (Some(lhs), None) => Some(lhs),
                (None, Some(rhs)) => Some(rhs),
                (Some(lhs), Some(rhs)) => Some(std::cmp::min(lhs, rhs)),
            }
        }

        Expr::FunctionCall(call) => match call.expr() {
            Some(expr) => find_smallest_range(expr),
            None => None,
        },

        Expr::NumberLiteral(_) => None,

        Expr::VectorSelector(sel) => sel.duration(),
    }
}
