use super::value::{ExprValue, ExprValueIter, ExprValueKind, InstantVector};
use crate::parser::ast::UnaryOp;

pub(super) struct UnaryExprExecutor {
    op: UnaryOp,
    inner: Box<dyn ExprValueIter>,
}

impl UnaryExprExecutor {
    pub fn new(op: UnaryOp, inner: Box<dyn ExprValueIter>) -> Self {
        Self { op, inner }
    }

    fn next_instant_vector(&self, mut v: InstantVector) -> ExprValue {
        if self.op == UnaryOp::Sub {
            v = v.apply_scalar_op(|v| Some(-v), true);
        }
        ExprValue::InstantVector(v)
    }
}

impl std::iter::Iterator for UnaryExprExecutor {
    type Item = ExprValue;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(ExprValue::InstantVector(v)) => Some(self.next_instant_vector(v)),
            None => None,
            _ => unimplemented!(),
        }
    }
}

impl ExprValueIter for UnaryExprExecutor {
    fn value_kind(&self) -> ExprValueKind {
        self.inner.value_kind()
    }
}
