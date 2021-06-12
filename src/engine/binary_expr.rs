use super::value::{ExprValue, ExprValueIter, ExprValueKind, InstantVector};
use crate::model::types::SampleValue;
use crate::parser::ast::{BinaryOp, GroupModifier, VectorMatching};

/// ArithmeticExprExecutor performs a binary operation between:
///   - scalar and scalar
///   - vector and scalar, or scalar and vector
///   - vector and vector.
pub(super) struct ArithmeticExprExecutor {
    op: BinaryOp,
    left: Box<dyn ExprValueIter>,
    right: Box<dyn ExprValueIter>,
    bool_modifier: bool,
    vector_matching: Option<VectorMatching>,
    group_modifier: Option<GroupModifier>,
}

impl ArithmeticExprExecutor {
    pub fn new(
        left: Box<dyn ExprValueIter>,
        op: BinaryOp,
        right: Box<dyn ExprValueIter>,
        bool_modifier: bool,
        vector_matching: Option<VectorMatching>,
        group_modifier: Option<GroupModifier>,
    ) -> Self {
        Self {
            op,
            left,
            right,
            bool_modifier,
            vector_matching,
            group_modifier,
        }
    }

    fn next_scalar_scalar(&self, lv: SampleValue, rv: SampleValue) -> ExprValue {
        use BinaryOp::*;

        ExprValue::Scalar(match self.op {
            Add => lv + rv,
            Div => lv / rv,
            Mul => lv * rv,
            Mod => lv % rv,
            Pow => SampleValue::powf(lv, rv),
            Sub => lv - rv,
            _ => unimplemented!(),
        })
    }

    fn next_scalar_vector(&self, n: SampleValue, mut v: InstantVector) -> ExprValue {
        use BinaryOp::*;

        match self.op {
            Add => v.mutate_values(|(_, val)| *val = n + *val),
            Div => v.mutate_values(|(_, val)| *val = n / *val),
            Mul => v.mutate_values(|(_, val)| *val = n * *val),
            Mod => v.mutate_values(|(_, val)| *val = n % *val),
            Pow => v.mutate_values(|(_, val)| *val = SampleValue::powf(n, *val)),
            Sub => v.mutate_values(|(_, val)| *val = n - *val),
            _ => unimplemented!(),
        }
        ExprValue::InstantVector(v)
    }

    fn next_vector_scalar(&self, mut v: InstantVector, n: SampleValue) -> ExprValue {
        use BinaryOp::*;

        match self.op {
            Add => v.mutate_values(|(_, val)| *val = *val + n),
            Div => v.mutate_values(|(_, val)| *val = *val / n),
            Mul => v.mutate_values(|(_, val)| *val = *val * n),
            Mod => v.mutate_values(|(_, val)| *val = *val % n),
            Pow => v.mutate_values(|(_, val)| *val = SampleValue::powf(*val, n)),
            Sub => v.mutate_values(|(_, val)| *val = *val - n),
            _ => unimplemented!(),
        }
        ExprValue::InstantVector(v)
    }

    fn next_vector_vector(
        &mut self,
        mut lv: InstantVector,
        mut rv: InstantVector,
    ) -> Option<ExprValue> {
        use BinaryOp::*;

        // Rather ugly way to align left and right vectors time-wise.
        // Making ValuiIter a peekable iterator could improve the readability
        // of this and similar cases.
        while lv.timestamp() != rv.timestamp() {
            lv = match self.left.next() {
                Some(ExprValue::InstantVector(lv)) => lv,
                None => return None,
                _ => panic!("bug"),
            };

            rv = match self.right.next() {
                Some(ExprValue::InstantVector(rv)) => rv,
                None => return None,
                _ => panic!("bug"),
            };
        }

        Some(ExprValue::InstantVector(lv.match_vector(
            &rv,
            vec![],
            vec![],
            |lval, rval| match self.op {
                Add => lval + rval,
                Div => lval / rval,
                Mul => lval * rval,
                Mod => lval % rval,
                Pow => SampleValue::powf(lval, rval),
                Sub => lval - rval,
                _ => unimplemented!(),
            },
        )))
    }
}

impl std::iter::Iterator for ArithmeticExprExecutor {
    type Item = ExprValue;

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
            (ExprValue::Scalar(lv), ExprValue::Scalar(rv)) => {
                return Some(self.next_scalar_scalar(lv, rv))
            }
            (ExprValue::Scalar(lv), ExprValue::InstantVector(rv)) => {
                return Some(self.next_scalar_vector(lv, rv))
            }
            (ExprValue::InstantVector(lv), ExprValue::Scalar(rv)) => {
                return Some(self.next_vector_scalar(lv, rv))
            }
            (ExprValue::InstantVector(lv), ExprValue::InstantVector(rv)) => {
                return self.next_vector_vector(lv, rv)
            }
            _ => unimplemented!(),
        }
    }
}

impl ExprValueIter for ArithmeticExprExecutor {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::Scalar
    }
}
