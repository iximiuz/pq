use std::collections::HashSet;

use super::parser::ast::FunctionName;
use super::value::{ExprValue, ExprValueIter, ExprValueKind, InstantVector, RangeVector};
use crate::model::{
    labels::{LabelValue, LabelsTrait},
    types::SampleValue,
};

pub(super) enum FuncCallArg {
    Number(f64),
    String(LabelValue),
    ValueIter(Box<dyn ExprValueIter>),
}

pub(super) fn create_func_call_executor(
    func_name: FunctionName,
    mut args: Vec<FuncCallArg>,
) -> Box<dyn ExprValueIter> {
    use FunctionName::*;

    match func_name {
        AvgOverTime | CountOverTime | LastOverTime | MinOverTime | MaxOverTime | SumOverTime => {
            assert!(args.len() == 1);
            if let Some(FuncCallArg::ValueIter(inner)) = args.pop() {
                return Box::new(AggOverTimeFuncExecutor::new(func_name, inner));
            }
            panic!("unexpected argument type");
        }
        _ => unimplemented!("Coming soon..."),
    }
}

struct AggOverTimeFuncExecutor {
    func_name: FunctionName,
    inner: Box<dyn ExprValueIter>,
}

impl AggOverTimeFuncExecutor {
    fn new(func_name: FunctionName, inner: Box<dyn ExprValueIter>) -> Self {
        Self { func_name, inner }
    }

    fn do_next(&self, v: RangeVector) -> InstantVector {
        use FunctionName::*;

        let samples = v
            .samples()
            .into_iter()
            .map(|(labels, values)| {
                (
                    labels.without(&HashSet::new()), // trick to remove __name__
                    match self.func_name {
                        AvgOverTime => {
                            values.iter().map(|(v, _)| *v).sum::<SampleValue>()
                                / values.len() as SampleValue
                        }
                        CountOverTime => values.len() as SampleValue,
                        LastOverTime => values.iter().last().unwrap().0,
                        MinOverTime => values
                            .iter()
                            .map(|(v, _)| *v)
                            .fold(SampleValue::INFINITY, |m, c| SampleValue::min(m, c)),
                        MaxOverTime => values
                            .iter()
                            .map(|(v, _)| *v)
                            .fold(SampleValue::NEG_INFINITY, |m, c| SampleValue::max(m, c)),
                        SumOverTime => values.iter().map(|(v, _)| *v).sum(),
                        _ => unreachable!("bug"),
                    },
                )
            })
            .collect();
        InstantVector::new(v.timestamp(), samples)
    }
}

impl std::iter::Iterator for AggOverTimeFuncExecutor {
    type Item = ExprValue;

    fn next(&mut self) -> Option<Self::Item> {
        use FunctionName::*;

        let v = match self.inner.next() {
            Some(ExprValue::RangeVector(v)) => v,
            None => return None,
            _ => unreachable!("bug"),
        };

        match self.func_name {
            AvgOverTime | CountOverTime | LastOverTime | MinOverTime | MaxOverTime
            | SumOverTime => Some(ExprValue::InstantVector(self.do_next(v))),
            _ => unreachable!(),
        }
    }
}

impl ExprValueIter for AggOverTimeFuncExecutor {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::InstantVector
    }
}
