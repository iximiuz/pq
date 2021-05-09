use crate::input::Input;
use crate::labels::MatchOp;
use crate::parser::ast::*;

pub struct Engine {}

impl Engine {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self, query: &AST, input: &mut Input) {
        match query.root {
            NodeKind::VectorSelector(ref selector) => loop {
                let record = match input.take_one().unwrap() {
                    Some(r) => r,
                    None => break,
                };

                let mut matched = true;
                for matcher in selector.matchers().iter() {
                    matched = match record.label(matcher.label()) {
                        Some(label) => match matcher.match_op() {
                            MatchOp::Eql => label == matcher.value(),
                            MatchOp::Neq => label != matcher.value(),
                            _ => unimplemented!(),
                        },
                        None => false,
                    };

                    if !matched {
                        break;
                    }
                }

                if matched {
                    println!("{:?}", record);
                }
            },
        }
    }
}
