use std::collections::{HashMap, HashSet};

use crate::model::{
    labels::{LabelName, Labels, LabelsTrait},
    types::{SampleValue, Timestamp},
};
use crate::parser::ast::LabelMatching;

// Every Expr can be evaluated to a value.
#[derive(Debug)]
pub enum ExprValue {
    InstantVector(InstantVector),
    RangeVector(RangeVector),
    Scalar(SampleValue),
    // String(String)
}

#[derive(Debug, PartialEq)]
pub(super) enum ExprValueKind {
    InstantVector,
    RangeVector,
    Scalar,
}

pub(super) trait ExprValueIter: std::iter::Iterator<Item = ExprValue> {
    fn value_kind(&self) -> ExprValueKind;
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

    // Used in scalar-to-vector binary op and in unary op.
    pub fn mutate_values(&mut self, f: impl FnMut(&mut (Labels, SampleValue))) {
        self.samples.iter_mut().for_each(f)
    }

    pub fn vector_match_one_to_one(
        &self,
        other: &InstantVector,
        bool_modifier: bool,
        drop_name: bool,
        label_matching: Option<&LabelMatching>,
        op: impl Fn(SampleValue, SampleValue) -> Option<SampleValue>,
    ) -> Self {
        assert!(self.instant == other.instant);

        // println!("LEFT VECTOR {:#?}", self);
        // println!("RIGHT VECTOR {:#?}", other);

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
                None => continue, // TODO: check bool_modifier
            };
            if !already_matched.insert(signature) {
                // TODO: replace with error
                panic!("Many-to-one matching detected! If it's desired, use explicit group_left/group_right modifier");
            }

            if drop_name {
                matched_labels.drop_name();
            }

            samples.push((matched_labels, sample));
        }

        InstantVector::new(self.instant, samples)
    }

    pub fn vector_match_one_to_many(
        &self,
        other: &InstantVector,
        _bool_modifier: bool,
        _label_matching: Option<&LabelMatching>,
        _include_labels: &Vec<LabelName>,
        _op: impl Fn(SampleValue, SampleValue) -> Option<SampleValue>,
    ) -> Self {
        assert!(self.instant == other.instant);
        unimplemented!();
    }

    pub fn vector_match_many_to_one(
        &self,
        other: &InstantVector,
        bool_modifier: bool,
        label_matching: Option<&LabelMatching>,
        include_labels: &Vec<LabelName>,
        op: impl Fn(SampleValue, SampleValue) -> Option<SampleValue>,
    ) -> Self {
        other.vector_match_one_to_many(
            self,
            bool_modifier,
            label_matching,
            include_labels,
            |l, r| op(r, l),
        )
    }
}

#[derive(Debug)]
pub struct RangeVector {}
