use super::value::{QueryValue, QueryValueIter, QueryValueKind};
use crate::model::SampleValue;

enum Inner {
    Scalar(SampleValue),
    // String(String),
}

pub struct IdentityEvaluator {
    val: Inner,
}

impl IdentityEvaluator {
    pub fn scalar(val: SampleValue) -> Self {
        Self {
            val: Inner::Scalar(val),
        }
    }
}

impl std::iter::Iterator for IdentityEvaluator {
    type Item = QueryValue;

    fn next(&mut self) -> Option<Self::Item> {
        match self.val {
            Inner::Scalar(val) => Some(QueryValue::Scalar(val)),
        }
    }
}

impl QueryValueIter for IdentityEvaluator {
    fn value_kind(&self) -> QueryValueKind {
        match self.val {
            Inner::Scalar(_) => QueryValueKind::Scalar,
        }
    }
}
