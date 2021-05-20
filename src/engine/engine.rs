use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::input::{Cursor, Input, Sample};
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

// Every Expr evaluates to a Value.
#[derive(Debug)]
enum Value {
    InstantVector(Rc<Sample>),
    RangeVector,
    Scalar,
}

type ValueIter = Box<dyn std::iter::Iterator<Item = Value>>;

pub struct Engine {}

impl Engine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, query: AST, input: Input, step: Duration) {
        let executor = Self::create_executor(query.root, input, step);
        for value in executor {
            println!("{:?}", value);
        }
    }

    fn create_executor(root: Expr, input: Input, step: Duration) -> ValueIter {
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

        let input = Rc::new(RefCell::new(input));
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
                            sel,
                            Input::cursor(Rc::clone(&input)),
                            step,
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
        // println!("UnaryExpr::new()");
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
        // println!("UnaryExpr::new()");
        Self { op, inner }
    }
}

impl std::iter::Iterator for UnaryExprExecutor {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(Value::InstantVector(s)) => Some(Value::InstantVector(Rc::new(Sample {
                name: s.name.clone(),
                value: match self.op {
                    UnaryOp::Add => s.value,
                    UnaryOp::Sub => -s.value,
                },
                timestamp: s.timestamp,
                labels: s.labels.clone(),
            }))),
            None => None,
            _ => unimplemented!(),
        }
    }
}

struct VectorSelectorExecutor {
    selector: VectorSelector,
    cursor: Rc<Cursor>,
    step: Duration,
}

impl VectorSelectorExecutor {
    fn new(selector: VectorSelector, cursor: Rc<Cursor>, step: Duration) -> Self {
        // println!("VectorSelector::new()");
        Self {
            selector,
            cursor: cursor,
            step: step,
        }
    }
}

impl std::iter::Iterator for VectorSelectorExecutor {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let sample = match self.cursor.read() {
                Some(s) => s,
                None => return None,
            };

            if self
                .selector
                .matchers()
                .iter()
                .all(|m| match sample.label(m.label()) {
                    Some(v) => m.matches(v),
                    None => false,
                })
            {
                return Some(Value::InstantVector(sample));
            }
        }
    }
}
