use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use super::binary_expr::create_binary_expr_executor;
use super::identity::IdentityExecutor;
use super::unary_expr::UnaryExprExecutor;
use super::value::ExprValueIter;
use super::vector::VectorSelectorExecutor;
use crate::common::time::TimeRange;
use crate::error::Result;
use crate::input::Input;
use crate::output::Output;
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

const DEFAULT_INTERVAL: Duration = Duration::from_millis(1000);
const DEFAULT_LOOKBACK: Duration = DEFAULT_INTERVAL;

pub struct Executor {
    input: Rc<RefCell<Input>>,
    output: RefCell<Output>,
    range: TimeRange,
    interval: Duration,
    lookback: Duration,
}

impl Executor {
    pub fn new(
        input: Input,
        output: Output,
        range: Option<TimeRange>,
        interval: Option<Duration>,
        lookback: Option<Duration>,
    ) -> Self {
        let interval = interval.unwrap_or(DEFAULT_INTERVAL);
        assert!(interval.as_secs() + (interval.subsec_nanos() as u64) > 0);

        Self {
            input: Rc::new(RefCell::new(input)),
            output: RefCell::new(output),
            range: range.unwrap_or(TimeRange::infinity()),
            interval,
            lookback: lookback.unwrap_or(DEFAULT_LOOKBACK),
        }
    }

    pub fn execute(&self, query: AST) -> Result<()> {
        // println!("Executor::execute {:#?}", query);

        for value in self.create_value_iter(query.root) {
            // TODO: if value iter is scalar, we need to wrap it into
            //       something that would produce a (timestamp, scalar) tuples
            //       instead.
            self.output.borrow_mut().write(&value)?;
        }
        // self.output.flush();
        Ok(())
    }

    fn create_value_iter(&self, node: Expr) -> Box<dyn ExprValueIter> {
        match node {
            Expr::UnaryExpr(op, expr) => {
                Box::new(UnaryExprExecutor::new(op, self.create_value_iter(*expr)))
            }

            Expr::BinaryExpr(expr) => {
                let (op, lhs, rhs, bool_modifier, vector_matching, group_modifier) =
                    expr.into_inner();
                create_binary_expr_executor(
                    op,
                    self.create_value_iter(*lhs),
                    self.create_value_iter(*rhs),
                    bool_modifier,
                    vector_matching,
                    group_modifier,
                )
            }

            // leaf node
            Expr::NumberLiteral(val) => Box::new(IdentityExecutor::scalar(val)),

            // leaf node
            Expr::VectorSelector(sel) => Box::new(VectorSelectorExecutor::new(
                Input::cursor(Rc::clone(&self.input)),
                sel,
                self.range,
                self.interval,
                self.lookback,
            )),

            _ => unimplemented!(),
        }

        // Alternative iterative implementation to consider:
        //
        // let mut queue = vec![(root, false)];
        // let mut stack: Vec<ValueIter> = vec![];

        // loop {
        //     let (node, seen) = match queue.pop() {
        //         Some((n, s)) => (n, s),
        //         None => break,
        //     };

        //     if !seen {
        //         match node {
        //             Expr::UnaryExpr(op, expr) => {
        //                 queue.push((Expr::UnaryExpr(op, Box::new(Expr::Noop)), true));
        //                 queue.push((*expr, false));
        //             }
        //             Expr::VectorSelector(sel) => {
        //                 stack.push(Box::new(VectorSelectorExecutor::new(
        //                     Input::cursor(Rc::clone(&self.input)),
        //                     sel,
        //                     self.range,
        //                     self.interval,
        //                     self.lookback,
        //                 )));
        //             }
        //             _ => unreachable!(),
        //         };
        //     } else {
        //         match node {
        //             Expr::UnaryExpr(op, _) => {
        //                 let inner = stack.pop().expect("must not be empty");
        //                 stack.push(Box::new(UnaryExprExecutor::new(op, inner)));
        //             }
        //             _ => unreachable!(),
        //         };
        //     }
        // }

        // assert!(stack.len() == 1);
        // stack.pop().unwrap()
    }
}
