use std::collections::{HashMap, HashSet};

use super::parser::ast::LabelMatching;
use crate::model::{LabelName, Labels, LabelsTrait, SampleValue, Timestamp};

// Every Expr can be evaluated to a value.
#[derive(Debug)]
pub enum QueryValue {
    InstantVector(InstantVector),
    RangeVector(RangeVector),
    Scalar(SampleValue),
    // String(String)
}

#[derive(Debug, PartialEq)]
pub(super) enum QueryValueKind {
    InstantVector,
    RangeVector,
    Scalar,
}

pub(super) trait QueryValueIter: std::iter::Iterator<Item = QueryValue> {
    fn value_kind(&self) -> QueryValueKind;
}

#[derive(Debug)]
pub struct InstantVector {
    instant: Timestamp,
    samples: Vec<(Labels, SampleValue)>,
}

impl InstantVector {
    pub fn new(instant: Timestamp, samples: Vec<(Labels, SampleValue)>) -> Self {
        Self { instant, samples }
    }

    #[inline]
    pub fn timestamp(&self) -> Timestamp {
        self.instant
    }

    #[inline]
    pub fn samples(&self) -> &[(Labels, SampleValue)] {
        return &self.samples;
    }

    pub fn apply_scalar_op(
        &mut self,
        op: impl Fn(SampleValue) -> Option<SampleValue>,
        keep_name: bool,
    ) -> Self {
        let samples = self
            .samples
            .iter()
            .cloned()
            .filter_map(|(mut labels, value)| match op(value) {
                Some(value) => {
                    if !keep_name {
                        labels.drop_name();
                    }
                    Some((labels, value))
                }
                None => None,
            })
            .collect();
        InstantVector::new(self.instant, samples)
    }

    pub fn apply_vector_op_one_to_one(
        &self,
        op: impl Fn(SampleValue, SampleValue) -> Option<SampleValue>,
        other: &InstantVector,
        label_matching: Option<&LabelMatching>,
        keep_name: bool,
    ) -> Self {
        assert!(self.instant == other.instant);

        let mut rhs = HashMap::new();
        for (labels, value) in other.samples.iter() {
            let matched_labels = match label_matching {
                Some(LabelMatching::On(names)) => labels.with(names),
                Some(LabelMatching::Ignoring(names)) => labels.without(names),
                None => labels.without(&HashSet::new()),
            };

            match rhs.insert(matched_labels.to_vec(), value) {
                Some(duplicate) => {
                    // TODO: replace with error
                    panic!(
                        "Found series collision for matchinng labels ({:?}).\nFirst: {:#?}\nSecond: {:#?}",
                        label_matching, duplicate, matched_labels
                    );
                }
                None => (),
            }
        }

        let mut samples = Vec::new();
        let mut already_matched = HashSet::new();
        for (labels, lvalue) in self.samples.iter() {
            let mut matched_labels = match label_matching {
                Some(LabelMatching::On(names)) => labels.with(names),
                Some(LabelMatching::Ignoring(names)) => labels.without(names),
                None => labels.without(&HashSet::new()),
            };

            let signature = matched_labels.to_vec();
            let rvalue = match rhs.get(&signature) {
                Some(rvalue) => rvalue,
                None => continue,
            };

            let sample = match op(*lvalue, **rvalue) {
                Some(sample) => sample,
                None => continue,
            };
            if !already_matched.insert(signature) {
                // TODO: replace with error
                panic!("Many-to-one matching detected! If it's desired, use explicit group_left/group_right modifier");
            }

            if keep_name {
                if let Some(name) = labels.name() {
                    matched_labels.set_name(name.to_string());
                }
            }

            samples.push((matched_labels, sample));
        }

        InstantVector::new(self.instant, samples)
    }

    pub fn apply_vector_op_one_to_many(
        &self,
        _op: impl Fn(SampleValue, SampleValue) -> Option<SampleValue>,
        other: &InstantVector,
        _label_matching: Option<&LabelMatching>,
        _include_labels: &Vec<LabelName>,
    ) -> Self {
        assert!(self.instant == other.instant);
        unimplemented!();
    }

    pub fn apply_vector_op_many_to_one(
        &self,
        op: impl Fn(SampleValue, SampleValue) -> Option<SampleValue>,
        other: &InstantVector,
        label_matching: Option<&LabelMatching>,
        include_labels: &Vec<LabelName>,
    ) -> Self {
        other.apply_vector_op_one_to_many(|l, r| op(r, l), self, label_matching, include_labels)
    }
}

#[derive(Debug)]
pub struct RangeVector {
    instant: Timestamp,
    samples: Vec<(Labels, Vec<(SampleValue, Timestamp)>)>,
}

impl RangeVector {
    pub fn new(instant: Timestamp, samples: Vec<(Labels, Vec<(SampleValue, Timestamp)>)>) -> Self {
        Self { instant, samples }
    }

    #[inline]
    pub fn timestamp(&self) -> Timestamp {
        self.instant
    }

    #[inline]
    pub fn samples(&self) -> &[(Labels, Vec<(SampleValue, Timestamp)>)] {
        return &self.samples;
    }
}
