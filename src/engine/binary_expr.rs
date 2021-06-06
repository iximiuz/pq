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

    fn next_scalar_vector(&self, n: Value, mut v: InstantVector) -> ValueKind {
        match self.op {
            BinaryOp::Add => v.mutate_values(|(_, val)| *val = n + *val),
            BinaryOp::Div => v.mutate_values(|(_, val)| *val = n / *val),
            BinaryOp::Mul => v.mutate_values(|(_, val)| *val = n * *val),
            BinaryOp::Mod => v.mutate_values(|(_, val)| *val = n % *val),
            BinaryOp::Pow => v.mutate_values(|(_, val)| *val = Value::powf(n, *val)),
            BinaryOp::Sub => v.mutate_values(|(_, val)| *val = n - *val),
            _ => unimplemented!(),
        }
        ValueKind::InstantVector(v)
    }

    fn next_vector_scalar(&self, mut v: InstantVector, n: Value) -> ValueKind {
        match self.op {
            BinaryOp::Add => v.mutate_values(|(_, val)| *val = *val + n),
            BinaryOp::Div => v.mutate_values(|(_, val)| *val = *val / n),
            BinaryOp::Mul => v.mutate_values(|(_, val)| *val = *val * n),
            BinaryOp::Mod => v.mutate_values(|(_, val)| *val = *val % n),
            BinaryOp::Pow => v.mutate_values(|(_, val)| *val = Value::powf(*val, n)),
            BinaryOp::Sub => v.mutate_values(|(_, val)| *val = *val - n),
            _ => unimplemented!(),
        }
        ValueKind::InstantVector(v)
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

        // println!("binary_expr\t{:?} {:?} {:?}", lv, self.op, rv);

        match (lv, rv) {
            (ValueKind::Scalar(lv), ValueKind::Scalar(rv)) => {
                return Some(self.next_scalar_scalar(lv, rv))
            }
            (ValueKind::Scalar(lv), ValueKind::InstantVector(rv)) => {
                return Some(self.next_scalar_vector(lv, rv))
            }
            (ValueKind::InstantVector(lv), ValueKind::Scalar(rv)) => {
                return Some(self.next_vector_scalar(lv, rv))
            }
            _ => unimplemented!(),
        }
    }
}
