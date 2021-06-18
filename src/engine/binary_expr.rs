use super::value::{ExprValue, ExprValueIter, ExprValueKind, InstantVector as Vector};
use crate::model::types::SampleValue;
use crate::parser::ast::{BinaryOp, BinaryOpKind, GroupModifier, LabelMatching};

pub(super) fn create_binary_expr_executor(
    op: BinaryOp,
    left: Box<dyn ExprValueIter>,
    right: Box<dyn ExprValueIter>,
    bool_modifier: bool,
    label_matching: Option<LabelMatching>,
    group_modifier: Option<GroupModifier>,
) -> Box<dyn ExprValueIter> {
    use BinaryOpKind::*;
    use ExprValueKind::*;

    match (left.value_kind(), op.kind(), right.value_kind()) {
        (Scalar, Arithmetic | Comparison, Scalar) => {
            assert!(Comparison != op.kind() || bool_modifier);
            assert!(label_matching.is_none());
            assert!(group_modifier.is_none());
            Box::new(BinaryExprExecutorScalarScalar::new(op, left, right))
        }
        (Scalar, Arithmetic | Comparison, InstantVector) => {
            assert!(!bool_modifier || Comparison == op.kind());
            assert!(label_matching.is_none());
            assert!(group_modifier.is_none());
            Box::new(BinaryExprExecutorScalarVector::new(
                op,
                left,
                right,
                bool_modifier,
            ))
        }
        (InstantVector, Arithmetic | Comparison, Scalar) => {
            assert!(!bool_modifier || Comparison == op.kind());
            assert!(label_matching.is_none());
            assert!(group_modifier.is_none());
            Box::new(BinaryExprExecutorVectorScalar::new(
                op,
                left,
                right,
                bool_modifier,
            ))
        }
        (InstantVector, Arithmetic | Comparison | Logical, InstantVector) => {
            assert!(!bool_modifier || Comparison == op.kind());
            assert!(group_modifier.is_none() || label_matching.is_some());
            Box::new(BinaryExprExecutorVectorVector::new(
                op,
                left,
                right,
                bool_modifier,
                label_matching,
                group_modifier,
            ))
        }
        (lk, ok, rk) => unimplemented!("{:?} {:?} {:?} operation is not supported", lk, ok, rk),
    }
}

/// BinaryExprExecutorScalarScalar
/// Ex:
///   1 + 2
///   42 / 6
///   2 ^ 64
///   ...
struct BinaryExprExecutorScalarScalar {
    op: BinaryOp,
    left: Box<dyn ExprValueIter>,
    right: Box<dyn ExprValueIter>,
}

impl BinaryExprExecutorScalarScalar {
    pub fn new(op: BinaryOp, left: Box<dyn ExprValueIter>, right: Box<dyn ExprValueIter>) -> Self {
        assert!(left.value_kind() == ExprValueKind::Scalar);
        assert!(right.value_kind() == ExprValueKind::Scalar);
        Self { op, left, right }
    }
}

impl std::iter::Iterator for BinaryExprExecutorScalarScalar {
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

impl ExprValueIter for BinaryExprExecutorScalarScalar {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::Scalar
    }
}

/// BinaryExprExecutorScalarVector
/// Ex:
///   2 * http_requests_total{}
///   42 - http_requests_total{method="GET"}
struct BinaryExprExecutorScalarVector {
    op: BinaryOp,
    left: Box<dyn ExprValueIter>,
    right: Box<dyn ExprValueIter>,
}

impl BinaryExprExecutorScalarVector {
    pub fn new(
        op: BinaryOp,
        left: Box<dyn ExprValueIter>,
        right: Box<dyn ExprValueIter>,
        bool_modifier: bool,
    ) -> Self {
        assert!(left.value_kind() == ExprValueKind::Scalar);
        assert!(right.value_kind() == ExprValueKind::InstantVector);
        Self { op, left, right }
    }
}

impl std::iter::Iterator for BinaryExprExecutorScalarVector {
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

impl ExprValueIter for BinaryExprExecutorScalarVector {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::InstantVector
    }
}

/// BinaryExprExecutorVectorScalar
/// Ex:
///   http_requests_total{} % 9000
///   http_requests_total{method="POST"} + 8
struct BinaryExprExecutorVectorScalar {
    op: BinaryOp,
    left: Box<dyn ExprValueIter>,
    right: Box<dyn ExprValueIter>,
}

impl BinaryExprExecutorVectorScalar {
    pub fn new(
        op: BinaryOp,
        left: Box<dyn ExprValueIter>,
        right: Box<dyn ExprValueIter>,
        bool_modifier: bool,
    ) -> Self {
        assert!(left.value_kind() == ExprValueKind::InstantVector);
        assert!(right.value_kind() == ExprValueKind::Scalar);
        Self { op, left, right }
    }
}

impl std::iter::Iterator for BinaryExprExecutorVectorScalar {
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

impl ExprValueIter for BinaryExprExecutorVectorScalar {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::InstantVector
    }
}

/// BinaryExprExecutorVectorVector
/// Ex:
///   http_requests_total{method="GET"} + http_requests_total{method="POST"}
///   http_requests_total{} + http_requests_content_length_bytes{} on (method, job)
///   http_requests_total{} + http_requests_content_length_bytes{} on (instance)
struct BinaryExprExecutorVectorVector {
    op: BinaryOp,
    left: std::iter::Peekable<Box<dyn ExprValueIter>>,
    right: std::iter::Peekable<Box<dyn ExprValueIter>>,
    label_matching: Option<LabelMatching>,
    group_modifier: Option<GroupModifier>,
}

impl BinaryExprExecutorVectorVector {
    pub fn new(
        op: BinaryOp,
        left: Box<dyn ExprValueIter>,
        right: Box<dyn ExprValueIter>,
        bool_modifier: bool,
        label_matching: Option<LabelMatching>,
        group_modifier: Option<GroupModifier>,
    ) -> Self {
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

impl std::iter::Iterator for BinaryExprExecutorVectorVector {
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
                let is_arithmetic = true;
                let bool_modifier = false;
                lv.vector_match_one_to_one(
                    &rv,
                    bool_modifier,
                    is_arithmetic || bool_modifier,
                    self.label_matching.as_ref(),
                    |ls, rs| Some(scalar_op_scalar(self.op, ls, rs)),
                )
            }
        }))
    }
}

impl ExprValueIter for BinaryExprExecutorVectorVector {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::InstantVector
    }
}

fn scalar_op_scalar(op: BinaryOp, lv: SampleValue, rv: SampleValue) -> SampleValue {
    use BinaryOp::*;

    match op {
        // Arithmetic
        Add => lv + rv,
        Div => lv / rv,
        Mul => lv * rv,
        Mod => lv % rv,
        Pow => SampleValue::powf(lv, rv),
        Sub => lv - rv,

        // Comparison
        Eql => (lv == rv) as i64 as SampleValue,
        Gte => (lv >= rv) as i64 as SampleValue,
        Gtr => (lv > rv) as i64 as SampleValue,
        Lss => (lv < rv) as i64 as SampleValue,
        Lte => (lv <= rv) as i64 as SampleValue,
        Neq => (lv != rv) as i64 as SampleValue,
        _ => unimplemented!(),
    }
}