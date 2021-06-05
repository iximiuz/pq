use crate::model::types::{Labels, Timestamp, Value};

// Every Expr can be evaluated to a Value.
#[derive(Debug)]
pub enum ValueKind {
    InstantVector(InstantVector),
    RangeVector(RangeVector),
    Scalar(Value),
}

pub(super) type ValueIter = Box<dyn std::iter::Iterator<Item = ValueKind>>;

#[derive(Debug)]
pub struct InstantVector {
    instant: Timestamp,
    samples: Vec<(Labels, Value)>,
}

impl InstantVector {
    pub fn new(instant: Timestamp, samples: Vec<(Labels, Value)>) -> Self {
        Self { instant, samples }
    }

    #[inline]
    pub fn timestamp(&self) -> Timestamp {
        self.instant
    }

    #[inline]
    pub fn samples(&self) -> &[(Labels, Value)] {
        return &self.samples;
    }

    pub fn mul(&mut self, m: Value) {
        self.samples.iter_mut().for_each(|(_, val)| *val = *val * m);
    }
}

#[derive(Debug)]
pub struct RangeVector {}
