use crate::model::types::{Labels, Timestamp, Value as ScalarValue};

// Every Expr can be evaluated to a Value.
#[derive(Debug)]
pub enum Value {
    InstantVector(InstantVector),
    RangeVector(RangeVector),
    Scalar(ScalarValue),
}

#[derive(Debug)]
pub struct InstantVector {
    instant: Timestamp,
    samples: Vec<(Labels, ScalarValue)>,
}

impl InstantVector {
    pub fn new(instant: Timestamp, samples: Vec<(Labels, ScalarValue)>) -> Self {
        Self { instant, samples }
    }

    #[inline]
    pub fn timestamp(&self) -> Timestamp {
        self.instant
    }

    #[inline]
    pub fn samples(&self) -> &[(Labels, ScalarValue)] {
        return &self.samples;
    }

    pub fn mul(&mut self, m: ScalarValue) {
        self.samples.iter_mut().for_each(|(_, val)| *val = *val * m);
    }
}

#[derive(Debug)]
pub struct RangeVector {}
