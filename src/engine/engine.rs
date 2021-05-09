use crate::input::Input;
use crate::parser::ast::*;

pub struct Engine {}

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
