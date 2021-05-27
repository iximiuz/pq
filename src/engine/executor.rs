use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use super::value::{InstantVector, Value};
use super::vector::VectorSelectorExecutor;
use crate::common::time::TimeRange;
use crate::input::{Input, Sample};
use crate::parser::ast::*;

// Simple use cases (filtration)
//
//     - Requests longer than 500ms
//     duration > 500ms
//
//     - Requests longer than 500ms intermixed with content_length matched by labels
//     duration > 500ms and content_length (but that's an advanced case)
//
//     - Requests bigger than 200 KB
//     content_length > 200
//     content_length > 200 and duration (but that's an advanced case)
//
//
// Advanced use cases (with resampling)
//
//     - RPS per series
//     rate(integral(duration > bool 0)[1s])
//
//     - RPS total
//     sum(rate(integral(duration > bool 0)[1s]))
//
//     - RPS by HTTP method
//     sum(rate(integral(duration > bool 0)[1s])) on "method"
//
//     - Throughput (MB/s) as a moving 5m window
//     rate(integral(content_length / (1024 * 1024))[5m])
//
//     - Request duration distribution
//     TODO: ...
//
// Advanced use cases require defining an evaluation step. I.e. every rate() calculation
// should be reported at some constant frequency (unlike the original samples that may
// appear at random times). Every aggregation such as sum() takes all the series (vertical
// axis) at a give sampling step and combines them. That's how different series are aligned
// in time. And since we define the time alignment, we can start combining instant vectors
// using the original Prometheus rules - by matching labels.

// Time axis is the horizontal one.
// Series axis is the vertical one.
//
// Range-vectors essentialy defines a moving time window.
//
// Horizontal functions accept range-vectors and do the aggregation over the time axis:
//   - rate
//   - increase
//   - delta
//   - <agg>_over_time
//   - ...
//
// Vertical functions accept instant-vectors and do modification of its values:
//   - abs
//   - ceil
//   - exp
//   - log
//   - ...
//
// [some] Operators accept instant-vectos and do the aggregation over the series axis:
//   - sum [on] - group time series
//   - min/max/avg/topk/bottomk
//   - count
//   - ...

type ValueIter = Box<dyn std::iter::Iterator<Item = Value>>;

const DEFAULT_INTERVAL: Duration = Duration::from_millis(1000);
const DEFAULT_LOOKBACK: Duration = DEFAULT_INTERVAL;

pub struct Executor {
    input: Rc<RefCell<Input>>,
    range: TimeRange,
    interval: Duration,
    lookback: Duration,
}

impl Executor {
    pub fn new(
        input: Input,
        range: Option<TimeRange>,
        interval: Option<Duration>,
        lookback: Option<Duration>,
    ) -> Self {
        let interval = interval.unwrap_or(DEFAULT_INTERVAL);
        assert!(interval.as_secs() + (interval.subsec_nanos() as u64) > 0);

        Self {
            input: Rc::new(RefCell::new(input)),
            range: range.unwrap_or(TimeRange::infinity()),
            interval,
            lookback: lookback.unwrap_or(DEFAULT_LOOKBACK),
        }
    }

    pub fn execute(&self, query: AST) {
        let iter = self.create_value_iter(query.root);
        for value in iter {
            println!("EXECUTOR VALUE {:?}", value);
        }
    }

    fn create_value_iter(&self, root: Expr) -> ValueIter {
        // Alternative recursive implementation:
        // match expr {
        //     Expr::BinaryExpr(left, op, right) => {
        //         let lhs = Self::create_executor(*left);
        //         let rhs = Self::create_executor(*right);
        //         Box::new(BinaryExpr::new(op, lhs, rhs))
        //     }
        //     Expr::UnaryExpr(op, expr) => {
        //         Box::new(UnaryExpr::new(op, Self::create_executor(*expr)))
        //     }
        //     // leaf node
        //     Expr::VectorSelector(selector) => Box::new(VectorSelector::new(selector)),
        //     _ => unimplemented!(),
        // }

        let mut queue = vec![(root, false)];
        let mut stack: Vec<ValueIter> = vec![];

        loop {
            let (node, seen) = match queue.pop() {
                Some((n, s)) => (n, s),
                None => break,
            };

            if !seen {
                match node {
                    Expr::UnaryExpr(op, expr) => {
                        queue.push((Expr::UnaryExpr(op, Box::new(Expr::Noop)), true));
                        queue.push((*expr, false));
                    }
                    Expr::VectorSelector(sel) => {
                        stack.push(Box::new(VectorSelectorExecutor::new(
                            Input::cursor(Rc::clone(&self.input)),
                            sel,
                            self.range,
                            self.interval,
                            self.lookback,
                        )));
                    }
                    _ => unreachable!(),
                };
            } else {
                match node {
                    Expr::UnaryExpr(op, _) => {
                        let inner = stack.pop().expect("must not be empty");
                        stack.push(Box::new(UnaryExprExecutor::new(op, inner)));
                    }
                    _ => unreachable!(),
                };
            }
        }

        assert!(stack.len() == 1);
        stack.pop().unwrap()
    }
}

struct BinaryExprExecutor {
    op: BinaryOp,
    left: ValueIter,
    right: ValueIter,
}

impl BinaryExprExecutor {
    fn new(op: BinaryOp, left: ValueIter, right: ValueIter) -> Self {
        Self { op, left, right }
    }
}

impl std::iter::Iterator for BinaryExprExecutor {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let lhs = match self.left.next() {
            Some(l) => l,
            None => return None,
        };

        let rhs = match self.right.next() {
            Some(r) => r,
            None => return None,
        };

        None

        // Some(Rc::new(Sample {
        //     name: format!("{}{:?}{}", lhs.name, self.op, rhs.name),
        //     value: match self.op {
        //         BinaryOp::Add => lhs.value + rhs.value,
        //         BinaryOp::Sub => lhs.value - rhs.value,
        //     },
        //     timestamp: lhs.timestamp,
        //     labels: lhs.labels.clone(),
        // }))
    }
}

struct UnaryExprExecutor {
    op: UnaryOp,
    inner: ValueIter,
}

impl UnaryExprExecutor {
    fn new(op: UnaryOp, inner: ValueIter) -> Self {
        Self { op, inner }
    }

    fn next_instant_vector(&self, v: InstantVector) -> Value {
        Value::InstantVector(v)
        // match self.op {
        //   UnaryOp::Add => s.value,
        //   UnaryOp::Sub => -s.value,
        // }
    }
}

impl std::iter::Iterator for UnaryExprExecutor {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(Value::InstantVector(v)) => Some(self.next_instant_vector(v)),
            None => None,
            _ => unimplemented!(),
        }
    }
}
