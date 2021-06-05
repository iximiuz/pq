use super::value::ValueKind;
use crate::model::types::Value;

enum Inner {
    Scalar(Value),
    // String(String),
}

pub struct IdentityExecutor {
    val: Inner,
}

impl IdentityExecutor {
    pub fn scalar(val: Value) -> Self {
        Self {
            val: Inner::Scalar(val),
        }
    }
}

impl std::iter::Iterator for IdentityExecutor {
    type Item = ValueKind;

    fn next(&mut self) -> Option<Self::Item> {
        match self.val {
            Inner::Scalar(val) => Some(ValueKind::Scalar(val)),
        }
    }
}
