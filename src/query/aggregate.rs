use std::collections::{BTreeMap, HashSet};

use super::parser::ast::{AggregateArgument, AggregateModifier, AggregateOp};
use super::value::{InstantVector, QueryValue, QueryValueIter, QueryValueKind};
use crate::model::{LabelsTrait, SampleValue};

pub(super) struct AggregateEvaluator {
    op: AggregateOp,
    inner: Box<dyn QueryValueIter>,
    modifier: Option<AggregateModifier>,
    argument: Option<AggregateArgument>,
}

impl AggregateEvaluator {
    pub(super) fn new(
        op: AggregateOp,
        inner: Box<dyn QueryValueIter>,
        modifier: Option<AggregateModifier>,
        argument: Option<AggregateArgument>,
    ) -> Self {
        assert!(inner.value_kind() == QueryValueKind::InstantVector);
        Self {
            op,
            inner,
            modifier,
            argument,
        }
    }

    fn apply_op(&self, agg_value: SampleValue, cur_value: SampleValue) -> SampleValue {
        use AggregateOp::*;

        match self.op {
            Max => SampleValue::max(agg_value, cur_value),
            Min => SampleValue::min(agg_value, cur_value),
            Sum => agg_value + cur_value,
            _ => unimplemented!("aggregation operator {:?} is not implemented yet", self.op),
        }
    }
}

impl std::iter::Iterator for AggregateEvaluator {
    type Item = QueryValue;

    fn next(&mut self) -> Option<Self::Item> {
        let v = match self.inner.next() {
            Some(QueryValue::InstantVector(v)) => v,
            None => return None,
            _ => unimplemented!(),
        };

        let mut agg = BTreeMap::new();
        for (labels, cur_value) in v.samples() {
            let agg_labels = match self.modifier {
                Some(AggregateModifier::By(ref names)) => labels.with(names),
                Some(AggregateModifier::Without(ref names)) => labels.without(names),
                None => labels.with(&HashSet::new()),
            };

            let signature = agg_labels.to_vec();

            match agg.remove(&signature) {
                Some((_, agg_value)) => {
                    agg.insert(
                        signature,
                        (agg_labels, self.apply_op(agg_value, *cur_value)),
                    );
                }
                None => {
                    agg.insert(signature, (agg_labels, *cur_value));
                }
            }
        }

        Some(QueryValue::InstantVector(InstantVector::new(
            v.timestamp(),
            agg.values().cloned().into_iter().collect(),
        )))
    }
}

impl QueryValueIter for AggregateEvaluator {
    fn value_kind(&self) -> QueryValueKind {
        self.inner.value_kind()
    }
}
