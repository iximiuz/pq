use std::collections::{BTreeMap, HashSet};

use super::value::{ExprValue, ExprValueIter, ExprValueKind, InstantVector};
use crate::model::{labels::LabelsTrait, types::SampleValue};
use crate::parser::ast::{AggregateArgument, AggregateModifier, AggregateOp};

pub(super) struct AggregateExprExecutor {
    op: AggregateOp,
    inner: Box<dyn ExprValueIter>,
    modifier: Option<AggregateModifier>,
    argument: Option<AggregateArgument>,
}

impl AggregateExprExecutor {
    pub(super) fn new(
        op: AggregateOp,
        inner: Box<dyn ExprValueIter>,
        modifier: Option<AggregateModifier>,
        argument: Option<AggregateArgument>,
    ) -> Self {
        assert!(inner.value_kind() == ExprValueKind::InstantVector);
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

impl std::iter::Iterator for AggregateExprExecutor {
    type Item = ExprValue;

    fn next(&mut self) -> Option<Self::Item> {
        let v = match self.inner.next() {
            Some(ExprValue::InstantVector(v)) => v,
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

        Some(ExprValue::InstantVector(InstantVector::new(
            v.timestamp(),
            agg.values().cloned().into_iter().collect(),
        )))
    }
}

impl ExprValueIter for AggregateExprExecutor {
    fn value_kind(&self) -> ExprValueKind {
        self.inner.value_kind()
    }
}
