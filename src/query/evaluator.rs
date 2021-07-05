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
const DEFAULT_LOOKBACK: Duration = DEFAULT_INTERVAL;

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
    ) -> Result<Self> {
        Ok(Self {
            inner: create_value_iter(&Context::new(records, interval, lookback, start_at), query),
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

        match self.inner.next() {
            Some(v) => Some(Ok(v)),
            None => None,
        }
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
        interval: Option<Duration>,
        lookback: Option<Duration>,
        start_at: Option<Timestamp>,
    ) -> Self {
        let interval = interval.unwrap_or(DEFAULT_INTERVAL);
        assert!(interval.as_secs() + (interval.subsec_nanos() as u64) > 0);

        Self {
            samples: Rc::new(RefCell::new(SampleReader::new(records))),
            interval,
            lookback: lookback.unwrap_or(DEFAULT_LOOKBACK),
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
