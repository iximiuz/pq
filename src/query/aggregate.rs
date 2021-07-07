use std::collections::{BTreeMap, BinaryHeap, HashSet};

use super::parser::ast::{AggregateArgument, AggregateModifier, AggregateOp};
use super::value::{InstantVector, QueryValue, QueryValueIter, QueryValueKind};
use crate::model::{Labels, LabelsTrait, SampleValue};

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

    fn apply_simple_op(&self, agg_value: SampleValue, cur_value: SampleValue) -> SampleValue {
        use AggregateOp::*;

        match self.op {
            Count => agg_value + 1.0,
            Group => 1.0,
            Max => SampleValue::max(agg_value, cur_value),
            Min => SampleValue::min(agg_value, cur_value),
            Sum => agg_value + cur_value,
            _ => unreachable!("bug"),
        }
    }

    fn next_simple(&self, v: InstantVector) -> InstantVector {
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
                        (agg_labels, self.apply_simple_op(agg_value, *cur_value)),
                    );
                }
                None => {
                    agg.insert(signature, (agg_labels, *cur_value));
                }
            }
        }

        InstantVector::new(v.timestamp(), agg.values().cloned().into_iter().collect())
    }

    // shamefully copy-pasted from next_simple().
    fn next_avg(&self, v: InstantVector) -> InstantVector {
        let mut agg = BTreeMap::new();
        for (labels, cur_value) in v.samples() {
            let agg_labels = match self.modifier {
                Some(AggregateModifier::By(ref names)) => labels.with(names),
                Some(AggregateModifier::Without(ref names)) => labels.without(names),
                None => labels.with(&HashSet::new()),
            };

            let signature = agg_labels.to_vec();

            match agg.remove(&signature) {
                Some((_, (sum, count))) => {
                    agg.insert(signature, (agg_labels, (sum + *cur_value, count + 1)));
                }
                None => {
                    agg.insert(signature, (agg_labels, (*cur_value, 1)));
                }
            }
        }

        InstantVector::new(
            v.timestamp(),
            agg.values()
                .map(|(labels, (sum, count))| (labels.clone(), sum / (*count) as SampleValue))
                .collect(),
        )
    }

    fn next_top_bottom_k(&self, v: InstantVector) -> InstantVector {
        use AggregateOp::*;

        assert!(self.op == TopK || self.op == BottomK);

        let k = match self.argument {
            Some(AggregateArgument::Number(k)) => k,
            _ => panic!("unexpected topk() or bottomk() first argument"),
        };

        #[derive(Clone, PartialEq)]
        struct Pair<'a>(AggregateOp, &'a Labels, SampleValue);

        impl<'a> std::cmp::Eq for Pair<'a> {}

        impl<'a> std::cmp::PartialOrd for Pair<'a> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl<'a> std::cmp::Ord for Pair<'a> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                match self.0 {
                    TopK => SampleValue::partial_cmp(&other.2, &self.2),
                    BottomK => SampleValue::partial_cmp(&self.2, &other.2),
                    _ => unreachable!("bug"),
                }
                .unwrap_or(std::cmp::Ordering::Equal)
            }
        }

        let mut agg: BTreeMap<Vec<u8>, BinaryHeap<Pair>> = BTreeMap::new();
        for (labels, cur_value) in v.samples() {
            let agg_labels = match self.modifier {
                Some(AggregateModifier::By(ref names)) => labels.with(names),
                Some(AggregateModifier::Without(ref names)) => labels.without(names),
                None => labels.with(&HashSet::new()),
            };

            let signature = agg_labels.to_vec();

            match agg.remove(&signature) {
                Some(mut top_elements) => {
                    top_elements.push(Pair(self.op, labels, *cur_value));
                    if top_elements.len() > k as usize {
                        top_elements.pop();
                    }

                    agg.insert(signature, top_elements);
                }
                None => {
                    let mut top_elements = BinaryHeap::new();
                    top_elements.push(Pair(self.op, labels, *cur_value));
                    agg.insert(signature, top_elements);
                }
            }
        }

        InstantVector::new(
            v.timestamp(),
            agg.values()
                .flat_map(|top| top.into_iter().map(|v| (v.1.clone(), v.2)))
                .collect(),
        )
    }
}

impl std::iter::Iterator for AggregateEvaluator {
    type Item = QueryValue;

    fn next(&mut self) -> Option<Self::Item> {
        use AggregateOp::*;

        let v = match self.inner.next() {
            Some(QueryValue::InstantVector(v)) => v,
            None => return None,
            _ => unimplemented!(),
        };

        Some(QueryValue::InstantVector(match self.op {
            Avg => self.next_avg(v),
            Count | Group | Max | Min | Sum => self.next_simple(v),
            TopK | BottomK => self.next_top_bottom_k(v),
            _ => unimplemented!("aggregation operator {:?} is not implemented yet", self.op),
        }))
    }
}

impl QueryValueIter for AggregateEvaluator {
    fn value_kind(&self) -> QueryValueKind {
        self.inner.value_kind()
    }
}
