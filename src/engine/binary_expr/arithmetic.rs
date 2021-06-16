use super::super::value::{ExprValue, ExprValueIter, ExprValueKind, InstantVector as Vector};
use crate::model::types::SampleValue;
use crate::parser::ast::{BinaryOp, BinaryOpKind, GroupModifier, LabelMatching};

/// ArithmeticExprExecutorScalarScalar
/// Ex:
///   1 + 2
///   42 / 6
///   2 ^ 64
///   ...
pub(super) struct ArithmeticExprExecutorScalarScalar {
    op: BinaryOp,
    left: Box<dyn ExprValueIter>,
    right: Box<dyn ExprValueIter>,
}

impl ArithmeticExprExecutorScalarScalar {
    pub fn new(op: BinaryOp, left: Box<dyn ExprValueIter>, right: Box<dyn ExprValueIter>) -> Self {
        assert!(op.kind() == BinaryOpKind::Arithmetic);
        assert!(left.value_kind() == ExprValueKind::Scalar);
        assert!(right.value_kind() == ExprValueKind::Scalar);
        Self { op, left, right }
    }
}

impl std::iter::Iterator for ArithmeticExprExecutorScalarScalar {
    type Item = ExprValue;

    fn next(&mut self) -> Option<Self::Item> {
        use ExprValue::*;

        let lv = match self.left.next() {
            Some(Scalar(lv)) => lv,
            None => return None,
            _ => unreachable!(),
        };

        let rv = match self.right.next() {
            Some(Scalar(rv)) => rv,
            None => return None,
            _ => unreachable!(),
        };

        Some(Scalar(scalar_op_scalar(self.op, lv, rv)))
    }
}

impl ExprValueIter for ArithmeticExprExecutorScalarScalar {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::Scalar
    }
}

/// ArithmeticExprExecutorScalarVector
/// Ex:
///   2 * http_requests_total{}
///   42 - http_requests_total{method="GET"}
pub(super) struct ArithmeticExprExecutorScalarVector {
    op: BinaryOp,
    left: Box<dyn ExprValueIter>,
    right: Box<dyn ExprValueIter>,
}

impl ArithmeticExprExecutorScalarVector {
    pub fn new(op: BinaryOp, left: Box<dyn ExprValueIter>, right: Box<dyn ExprValueIter>) -> Self {
        assert!(op.kind() == BinaryOpKind::Arithmetic);
        assert!(left.value_kind() == ExprValueKind::Scalar);
        assert!(right.value_kind() == ExprValueKind::InstantVector);
        Self { op, left, right }
    }
}

impl std::iter::Iterator for ArithmeticExprExecutorScalarVector {
    type Item = ExprValue;

    fn next(&mut self) -> Option<Self::Item> {
        use ExprValue::*;

        let lv = match self.left.next() {
            Some(Scalar(lv)) => lv,
            None => return None,
            _ => unreachable!(),
        };

        let mut rv = match self.right.next() {
            Some(InstantVector(rv)) => rv,
            None => return None,
            _ => unreachable!(),
        };

        rv.mutate_values(|(_, val)| {
            *val = scalar_op_scalar(self.op, lv, *val);
        });
        Some(InstantVector(rv))
    }
}

impl ExprValueIter for ArithmeticExprExecutorScalarVector {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::InstantVector
    }
}

/// ArithmeticExprExecutorVectorScalar
/// Ex:
///   http_requests_total{} % 9000
///   http_requests_total{method="POST"} + 8
pub(super) struct ArithmeticExprExecutorVectorScalar {
    op: BinaryOp,
    left: Box<dyn ExprValueIter>,
    right: Box<dyn ExprValueIter>,
}

impl ArithmeticExprExecutorVectorScalar {
    pub fn new(op: BinaryOp, left: Box<dyn ExprValueIter>, right: Box<dyn ExprValueIter>) -> Self {
        assert!(op.kind() == BinaryOpKind::Arithmetic);
        assert!(left.value_kind() == ExprValueKind::InstantVector);
        assert!(right.value_kind() == ExprValueKind::Scalar);
        Self { op, left, right }
    }
}

impl std::iter::Iterator for ArithmeticExprExecutorVectorScalar {
    type Item = ExprValue;

    fn next(&mut self) -> Option<Self::Item> {
        use ExprValue::*;

        let mut lv = match self.left.next() {
            Some(InstantVector(lv)) => lv,
            None => return None,
            _ => unreachable!(),
        };

        let rv = match self.right.next() {
            Some(Scalar(rv)) => rv,
            None => return None,
            _ => unreachable!(),
        };

        lv.mutate_values(|(_, val)| {
            *val = scalar_op_scalar(self.op, *val, rv);
        });
        Some(InstantVector(lv))
    }
}

impl ExprValueIter for ArithmeticExprExecutorVectorScalar {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::InstantVector
    }
}

/// ArithmeticExprExecutorVectorVector
/// Ex:
///   http_requests_total{method="GET"} + http_requests_total{method="POST"}
///   http_requests_total{} + http_requests_content_length_bytes{} on (method, job)
///   http_requests_total{} + http_requests_content_length_bytes{} on (instance)
pub(super) struct ArithmeticExprExecutorVectorVector {
    op: BinaryOp,
    left: std::iter::Peekable<Box<dyn ExprValueIter>>,
    right: std::iter::Peekable<Box<dyn ExprValueIter>>,
    label_matching: Option<LabelMatching>,
    group_modifier: Option<GroupModifier>,
}

impl ArithmeticExprExecutorVectorVector {
    pub fn new(
        op: BinaryOp,
        left: Box<dyn ExprValueIter>,
        right: Box<dyn ExprValueIter>,
        label_matching: Option<LabelMatching>,
        group_modifier: Option<GroupModifier>,
    ) -> Self {
        assert!(op.kind() == BinaryOpKind::Arithmetic);
        assert!(left.value_kind() == ExprValueKind::InstantVector);
        assert!(right.value_kind() == ExprValueKind::InstantVector);
        Self {
            op,
            left: left.peekable(),
            right: right.peekable(),
            label_matching: label_matching,
            group_modifier: group_modifier,
        }
    }
}

impl std::iter::Iterator for ArithmeticExprExecutorVectorVector {
    type Item = ExprValue;

    fn next(&mut self) -> Option<Self::Item> {
        use ExprValue::*;

        // Only aligned in time vectors can be matched.

        let (lv, rv) = loop {
            let (lv, rv) = match (self.left.peek(), self.right.peek()) {
                (Some(InstantVector(lv)), Some(InstantVector(rv))) => (lv, rv),
                (None, _) | (_, None) => return None,
                _ => unreachable!(),
            };

            if lv.timestamp() == rv.timestamp() {
                break match (self.left.next(), self.right.next()) {
                    (Some(InstantVector(lv)), Some(InstantVector(rv))) => (lv, rv),
                    _ => unreachable!(),
                };
            }

            let (ltimestamp, rtimestamp) = (lv.timestamp(), rv.timestamp());
            if ltimestamp < rtimestamp {
                // left vector is behind right vector in time.
                // consume left one, but produce no result yet
                self.left.next().unwrap();
            } else {
                // right vector is behind left vector in time.
                // consume rigth one, but produce no result yet
                self.right.next().unwrap();
            }

            return Some(InstantVector(Vector::new(
                std::cmp::min(ltimestamp, rtimestamp),
                vec![],
            )));
        };

        Some(InstantVector(match self.group_modifier {
            Some(GroupModifier::Left(ref include)) => lv.vector_match_many_to_one(
                &rv,
                false,
                self.label_matching.as_ref(),
                include,
                |ls, rs| Some(scalar_op_scalar(self.op, ls, rs)),
            ),
            Some(GroupModifier::Right(ref include)) => lv.vector_match_one_to_many(
                &rv,
                false,
                self.label_matching.as_ref(),
                include,
                |ls, rs| Some(scalar_op_scalar(self.op, ls, rs)),
            ),
            None => {
                lv.vector_match_one_to_one(&rv, false, self.label_matching.as_ref(), |ls, rs| {
                    Some(scalar_op_scalar(self.op, ls, rs))
                })
            }
        }))
    }
}

impl ExprValueIter for ArithmeticExprExecutorVectorVector {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::InstantVector
    }
}

fn scalar_op_scalar(op: BinaryOp, lv: SampleValue, rv: SampleValue) -> SampleValue {
    use BinaryOp::*;

    match op {
        Add => lv + rv,
        Div => lv / rv,
        Mul => lv * rv,
        Mod => lv % rv,
        Pow => SampleValue::powf(lv, rv),
        Sub => lv - rv,
        _ => unimplemented!(),
    }
}
