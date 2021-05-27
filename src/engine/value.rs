use crate::model::types::{Timestamp, Value as ScalarValue};

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
}

impl InstantVector {
    pub fn new(instant: Timestamp) -> Self {
        Self { instant }
    }
}

#[derive(Debug)]
pub(super) struct RangeVector {}
