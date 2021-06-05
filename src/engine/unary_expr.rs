use super::value::{InstantVector, ValueIter, ValueKind};
use crate::parser::ast::UnaryOp;

pub struct UnaryExprExecutor {
    op: UnaryOp,
    inner: ValueIter,
}

impl UnaryExprExecutor {
    pub fn new(op: UnaryOp, inner: ValueIter) -> Self {
        Self { op, inner }
    }

    fn next_instant_vector(&self, mut v: InstantVector) -> ValueKind {
        if self.op == UnaryOp::Sub {
            v.mul(-1.0);
        }
        ValueKind::InstantVector(v)
    }
}

impl std::iter::Iterator for UnaryExprExecutor {
    type Item = ValueKind;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(ValueKind::InstantVector(v)) => Some(self.next_instant_vector(v)),
            None => None,
            _ => unimplemented!(),
        }
    }
}
