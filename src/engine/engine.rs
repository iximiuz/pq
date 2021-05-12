use crate::input::Input;
use crate::parser::ast::*;

pub struct Engine {}

// - Requests longer than 500ms
// duration > 500ms
//
// - Requests longer than 500ms intermixed with content_length matched by labels
// duration > 500ms and content_length
//
// - Requests bigger than 200 KB
// content_length > 200
// content_length > 200 and duration
//
// - RPS per series
// rate(integral(duration > bool 0)[1s])
//
// - RPS total
// sum(rate(integral(duration > bool 0)[1s]))
//
// - RPS by HTTP method
// sum(rate(integral(duration > bool 0)[1s])) on "method"
//
// - Throughput (MB/s) as a moving 5m window
// rate(integral(content_length / (1024 * 1024))[5m])
//
// - Request duration distribution
// TODO: ...

impl Engine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, query: &AST, input: &mut Input) {
        match query.root {
            Expr::VectorSelector(ref selector) => loop {
                let record = match input.take_one().unwrap() {
                    Some(r) => r,
                    None => return,
                };

                for metric in record.metrics() {
                    let matched =
                        selector
                            .matchers()
                            .iter()
                            .all(|m| match metric.label(m.label()) {
                                Some(v) => m.matches(v),
                                None => false,
                            });

                    if matched {
                        println!("{:?}", record);
                    }
                }
            },
            Expr::UnaryExpr(_, _) => unimplemented!(),
        }
    }
}
