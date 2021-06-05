use super::value::{InstantVector, ValueIter, ValueKind};
use crate::model::types::Value;
use crate::parser::ast::BinaryOp;

/// BinaryExprExecutor performs a binary operation between:
///   - scalar and scalar
///   - vector and scalar, or scalar and vector
///   - vector and vector.
pub struct BinaryExprExecutor {
    op: BinaryOp,
    left: ValueIter,
    right: ValueIter,
}

impl BinaryExprExecutor {
    pub fn new(op: BinaryOp, left: ValueIter, right: ValueIter) -> Self {
        Self { op, left, right }
    }

    fn next_scalar_scalar(&self, lv: Value, rv: Value) -> ValueKind {
        ValueKind::Scalar(match self.op {
            BinaryOp::Add => lv + rv,
            BinaryOp::Div => lv / rv,
            BinaryOp::Mul => lv * rv,
            BinaryOp::Mod => lv % rv,
            BinaryOp::Pow => Value::powf(lv, rv),
            BinaryOp::Sub => lv - rv,
            _ => unimplemented!(),
        })
    }

    fn next_scalar_vector(&self, lv: Value, mut rv: InstantVector) -> ValueKind {
        match self.op {
            BinaryOp::Add => rv.mutate_values(|(_, val)| *val = lv + *val),
            BinaryOp::Sub => rv.mutate_values(|(_, val)| *val = lv - *val),
            _ => unimplemented!(),
        }
        ValueKind::InstantVector(rv)
    }
}

impl std::iter::Iterator for BinaryExprExecutor {
    type Item = ValueKind;

    fn next(&mut self) -> Option<Self::Item> {
        let lv = match self.left.next() {
            Some(lv) => lv,
            None => return None,
        };

        let rv = match self.right.next() {
            Some(rv) => rv,
            None => return None,
        };

        match (lv, rv) {
            (ValueKind::Scalar(lv), ValueKind::Scalar(rv)) => {
                return Some(self.next_scalar_scalar(lv, rv))
            }
            (ValueKind::Scalar(lv), ValueKind::InstantVector(rv)) => {
                return Some(self.next_scalar_vector(lv, rv))
            }
            // (ValueKind::InstantVector(lv), ValueKind::Scalar(rv)) => {
            //     return Some(self.next_vector_scalar(lv, rv))
            // }
            _ => unimplemented!(),
        }
    }
}
