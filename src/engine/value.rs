use crate::model::types::{Labels, Timestamp, Value as ScalarValue};

// Every Expr can be evaluated to a Value.
#[derive(Debug)]
pub(super) enum Value {
    InstantVector(InstantVector),
    RangeVector(RangeVector),
    Scalar(ScalarValue),
}

#[derive(Debug)]
pub(super) struct InstantVector {
    instant: Timestamp,
    samples: Vec<(Labels, ScalarValue)>,
}

impl InstantVector {
    pub fn new(instant: Timestamp, samples: Vec<(Labels, ScalarValue)>) -> Self {
        Self { instant, samples }
    }

    pub fn mul(&mut self, m: ScalarValue) {
        self.samples.iter_mut().for_each(|(_, val)| *val = *val * m);
    }
}

#[derive(Debug)]
pub(super) struct RangeVector {}
