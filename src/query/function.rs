use std::collections::HashSet;

use super::parser::ast::FunctionName;
use super::value::{InstantVector, QueryValue, QueryValueIter, QueryValueKind, RangeVector};
use crate::model::{LabelValue, LabelsTrait, SampleValue};

pub(super) enum FuncCallArg {
    Number(f64),
    String(LabelValue),
    ValueIter(Box<dyn QueryValueIter>),
}

pub(super) fn create_func_evaluator(
    func_name: FunctionName,
    mut args: Vec<FuncCallArg>,
) -> Box<dyn QueryValueIter> {
    use FunctionName::*;

    match func_name {
        AvgOverTime | CountOverTime | LastOverTime | MinOverTime | MaxOverTime | SumOverTime => {
            assert!(args.len() == 1);
            if let Some(FuncCallArg::ValueIter(inner)) = args.pop() {
                return Box::new(AggOverTimeFuncEvaluator::new(func_name, inner));
            }
            panic!("unexpected argument type");
        }
        _ => unimplemented!("Coming soon..."),
    }
}

struct AggOverTimeFuncEvaluator {
    func_name: FunctionName,
    inner: Box<dyn QueryValueIter>,
}

impl AggOverTimeFuncEvaluator {
    fn new(func_name: FunctionName, inner: Box<dyn QueryValueIter>) -> Self {
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

impl std::iter::Iterator for AggOverTimeFuncEvaluator {
    type Item = QueryValue;

    fn next(&mut self) -> Option<Self::Item> {
        use FunctionName::*;

        let v = match self.inner.next() {
            Some(QueryValue::RangeVector(v)) => v,
            None => return None,
            _ => unreachable!("bug"),
        };

        match self.func_name {
            AvgOverTime | CountOverTime | LastOverTime | MinOverTime | MaxOverTime
            | SumOverTime => Some(QueryValue::InstantVector(self.do_next(v))),
            _ => unreachable!(),
        }
    }
}

impl QueryValueIter for AggOverTimeFuncEvaluator {
    fn value_kind(&self) -> QueryValueKind {
        QueryValueKind::InstantVector
    }
}
