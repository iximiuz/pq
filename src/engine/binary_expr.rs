use super::value::{ValueIter, ValueKind};
use crate::model::types::Value;
use crate::parser::ast::BinaryOp;

/// BinaryExprExecutor performs a binary operation between:
///   - scalar and scalar
///   - vector and scalar
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

    fn next_scalar_scalar(&self, l: Value, r: Value) -> ValueKind {
        ValueKind::Scalar(match self.op {
            BinaryOp::Add => l + r,
            BinaryOp::Div => l / r,
            BinaryOp::Mul => l * r,
            BinaryOp::Mod => l % r,
            BinaryOp::Pow => Value::powf(l, r),
            BinaryOp::Sub => l - r,
            _ => unimplemented!(),
        })
    }
}

impl std::iter::Iterator for BinaryExprExecutor {
    type Item = ValueKind;

    fn next(&mut self) -> Option<Self::Item> {
        let lhs = match self.left.next() {
            Some(l) => l,
            None => return None,
        };

        let rhs = match self.right.next() {
            Some(r) => r,
            None => return None,
        };

        None

        // Some(Rc::new(Sample {
        //     name: format!("{}{:?}{}", lhs.name, self.op, rhs.name),
        //     value: match self.op {
        //         BinaryOp::Add => lhs.value + rhs.value,
        //         BinaryOp::Sub => lhs.value - rhs.value,
        //     },
        //     timestamp: lhs.timestamp,
        //     labels: lhs.labels.clone(),
        // }))
    }
}
