use super::value::{ExprValue, ExprValueIter, ExprValueKind};
use crate::model::SampleValue;

enum Inner {
    Scalar(SampleValue),
    // String(String),
}

pub struct IdentityExecutor {
    val: Inner,
}

impl IdentityExecutor {
    pub fn scalar(val: SampleValue) -> Self {
        Self {
            val: Inner::Scalar(val),
        }
    }
}

impl std::iter::Iterator for IdentityExecutor {
    type Item = ExprValue;

    fn next(&mut self) -> Option<Self::Item> {
        match self.val {
            Inner::Scalar(val) => Some(ExprValue::Scalar(val)),
        }
    }
}

impl ExprValueIter for IdentityExecutor {
    fn value_kind(&self) -> ExprValueKind {
        match self.val {
            Inner::Scalar(_) => ExprValueKind::Scalar,
        }
    }
}
