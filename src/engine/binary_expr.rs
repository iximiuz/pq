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
        use BinaryOp::*;

        ValueKind::Scalar(match self.op {
            Add => lv + rv,
            Div => lv / rv,
            Mul => lv * rv,
            Mod => lv % rv,
            Pow => Value::powf(lv, rv),
            Sub => lv - rv,
            _ => unimplemented!(),
        })
    }

    fn next_scalar_vector(&self, n: Value, mut v: InstantVector) -> ValueKind {
        use BinaryOp::*;

        match self.op {
            Add => v.mutate_values(|(_, val)| *val = n + *val),
            Div => v.mutate_values(|(_, val)| *val = n / *val),
            Mul => v.mutate_values(|(_, val)| *val = n * *val),
            Mod => v.mutate_values(|(_, val)| *val = n % *val),
            Pow => v.mutate_values(|(_, val)| *val = Value::powf(n, *val)),
            Sub => v.mutate_values(|(_, val)| *val = n - *val),
            _ => unimplemented!(),
        }
        ValueKind::InstantVector(v)
    }

    fn next_vector_scalar(&self, mut v: InstantVector, n: Value) -> ValueKind {
        use BinaryOp::*;

        match self.op {
            Add => v.mutate_values(|(_, val)| *val = *val + n),
            Div => v.mutate_values(|(_, val)| *val = *val / n),
            Mul => v.mutate_values(|(_, val)| *val = *val * n),
            Mod => v.mutate_values(|(_, val)| *val = *val % n),
            Pow => v.mutate_values(|(_, val)| *val = Value::powf(*val, n)),
            Sub => v.mutate_values(|(_, val)| *val = *val - n),
            _ => unimplemented!(),
        }
        ValueKind::InstantVector(v)
    }

    fn next_vector_vector(
        &mut self,
        mut lv: InstantVector,
        mut rv: InstantVector,
    ) -> Option<ValueKind> {
        use BinaryOp::*;

        // Rather ugly way to align left and right vectors time-wise.
        // Making ValuiIter a peekable iterator could improve the readability
        // of this and similar cases.
        while lv.timestamp() != rv.timestamp() {
            lv = match self.left.next() {
                Some(ValueKind::InstantVector(lv)) => lv,
                None => return None,
                _ => panic!("bug"),
            };

            rv = match self.right.next() {
                Some(ValueKind::InstantVector(rv)) => rv,
                None => return None,
                _ => panic!("bug"),
            };
        }

        Some(ValueKind::InstantVector(lv.match_vector(
            &rv,
            vec![],
            vec![],
            |lval, rval| match self.op {
                Add => lval + rval,
                Div => lval / rval,
                Mul => lval * rval,
                Mod => lval % rval,
                Pow => Value::powf(lval, rval),
                Sub => lval - rval,
                _ => unimplemented!(),
            },
        )))
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
            (ValueKind::InstantVector(lv), ValueKind::InstantVector(rv)) => {
                return self.next_vector_vector(lv, rv)
            }
            _ => unimplemented!(),
        }
    }
}
