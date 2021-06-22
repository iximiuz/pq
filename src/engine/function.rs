use super::value::{ExprValue, ExprValueIter, ExprValueKind};
use crate::model::{labels::LabelValue, types::SampleValue};
use crate::parser::ast::FunctionName;

pub(super) enum FuncCallArg {
    Number(f64),
    String(LabelValue),
    ValueIter(Box<dyn ExprValueIter>),
}

pub(super) fn create_func_call_executor(
    func_name: FunctionName,
    args: Vec<FuncCallArg>,
) -> Box<dyn ExprValueIter> {
    Box::new(AggOverTimeFuncExecutor::new())
}

struct AggOverTimeFuncExecutor {}

impl AggOverTimeFuncExecutor {
    fn new() -> Self {
        Self {}
    }
}

impl std::iter::Iterator for AggOverTimeFuncExecutor {
    type Item = ExprValue;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

impl ExprValueIter for AggOverTimeFuncExecutor {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::InstantVector
    }
}
