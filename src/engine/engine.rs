use std::rc::Rc;

use crate::input::{Input, Sample};
use crate::parser::ast;

pub struct Engine {}

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

impl Engine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, query: ast::AST, input: &mut Input) {
        for sample in self.do_execute(query.root, Box::new(input.cursor())) {
            println!("{:?}", sample);
        }
    }

    fn do_execute<'a>(
        &self,
        expr: ast::Expr,
        input: Box<dyn std::iter::Iterator<Item = Rc<Sample>> + 'a>,
    ) -> Box<dyn std::iter::Iterator<Item = Rc<Sample>> + 'a> {
        match expr {
            ast::Expr::BinaryExpr(left, op, right) => Box::new(BinaryExpr::new(
                self.do_execute(*left, input),
                op,
                self.do_execute(*right, input),
            )),
            ast::Expr::UnaryExpr(op, expr) => {
                Box::new(UnaryExpr::new(op, self.do_execute(*expr, input)))
            }
            ast::Expr::VectorSelector(selector) => Box::new(VectorSelector::new(selector, input)),
        }
    }
}

struct BinaryExpr<'a> {
    op: ast::BinaryOp,
    left: Box<dyn std::iter::Iterator<Item = Rc<Sample>> + 'a>,
    right: Box<dyn std::iter::Iterator<Item = Rc<Sample>> + 'a>,
}

impl<'a> BinaryExpr<'a> {
    fn new(
        left: Box<dyn std::iter::Iterator<Item = Rc<Sample>> + 'a>,
        op: ast::BinaryOp,
        right: Box<dyn std::iter::Iterator<Item = Rc<Sample>> + 'a>,
    ) -> Self {
        // println!("UnaryExpr::new()");
        BinaryExpr { op, left, right }
    }
}

impl<'a> std::iter::Iterator for BinaryExpr<'a> {
    type Item = Rc<Sample>;

    fn next(&mut self) -> Option<Self::Item> {
        let lhs = match self.left.next() {
            Some(l) => l,
            None => return None,
        };

        let rhs = match self.right.next() {
            Some(r) => r,
            None => return None,
        };

        Some(Rc::new(Sample {
            name: format!("{}{:?}{}", lhs.name, self.op, rhs.name),
            value: match self.op {
                ast::BinaryOp::Add => lhs.value + rhs.value,
                ast::BinaryOp::Sub => lhs.value - rhs.value,
            },
            timestamp: lhs.timestamp,
            labels: lhs.labels.clone(),
        }))
    }
}

struct UnaryExpr<'a> {
    op: ast::UnaryOp,
    inner: Box<dyn std::iter::Iterator<Item = Rc<Sample>> + 'a>,
}

impl<'a> UnaryExpr<'a> {
    fn new(op: ast::UnaryOp, inner: Box<dyn std::iter::Iterator<Item = Rc<Sample>> + 'a>) -> Self {
        // println!("UnaryExpr::new()");
        UnaryExpr { op, inner }
    }
}

impl<'a> std::iter::Iterator for UnaryExpr<'a> {
    type Item = Rc<Sample>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(s) => Some(Rc::new(Sample {
                name: s.name.clone(),
                value: match self.op {
                    ast::UnaryOp::Add => s.value,
                    ast::UnaryOp::Sub => -s.value,
                },
                timestamp: s.timestamp,
                labels: s.labels.clone(),
            })),
            None => None,
        }
    }
}

struct VectorSelector<'a> {
    selector: ast::VectorSelector,
    inner: Box<dyn std::iter::Iterator<Item = Rc<Sample>> + 'a>,
}

impl<'a> VectorSelector<'a> {
    fn new(
        selector: ast::VectorSelector,
        inner: Box<dyn std::iter::Iterator<Item = Rc<Sample>> + 'a>,
    ) -> Self {
        // println!("VectorSelector::new()");
        VectorSelector { selector, inner }
    }
}

impl<'a> std::iter::Iterator for VectorSelector<'a> {
    type Item = Rc<Sample>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let sample = match self.inner.next() {
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
                return Some(sample);
            }
        }
    }
}
