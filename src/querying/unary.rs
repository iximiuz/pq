use super::parser::ast::UnaryOp;
use super::value::{InstantVector, QueryValue, QueryValueIter, QueryValueKind};

pub(super) struct UnaryEvaluator {
    op: UnaryOp,
    inner: Box<dyn QueryValueIter>,
}

impl UnaryEvaluator {
    pub fn new(op: UnaryOp, inner: Box<dyn QueryValueIter>) -> Self {
        Self { op, inner }
    }

    fn next_instant_vector(&self, mut v: InstantVector) -> QueryValue {
        if self.op == UnaryOp::Sub {
            v = v.apply_scalar_op(|v| Some(-v), true);
        }
        QueryValue::InstantVector(v)
    }
}

impl std::iter::Iterator for UnaryEvaluator {
    type Item = QueryValue;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(QueryValue::InstantVector(v)) => Some(self.next_instant_vector(v)),
            None => None,
            _ => unimplemented!(),
        }
    }
}

impl QueryValueIter for UnaryEvaluator {
    fn value_kind(&self) -> QueryValueKind {
        self.inner.value_kind()
    }
}
