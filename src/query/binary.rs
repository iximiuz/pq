use super::parser::ast::{BinaryOp, BinaryOpKind, GroupModifier, LabelMatching};
use super::value::{InstantVector as Vector, QueryValue, QueryValueIter, QueryValueKind};
use crate::model::SampleValue;

pub(super) fn create_binary_evaluator(
    op: BinaryOp,
    left: Box<dyn QueryValueIter>,
    right: Box<dyn QueryValueIter>,
    bool_modifier: bool,
    label_matching: Option<LabelMatching>,
    group_modifier: Option<GroupModifier>,
) -> Box<dyn QueryValueIter> {
    use BinaryOpKind::*;
    use QueryValueKind::*;

    match (left.value_kind(), op.kind(), right.value_kind()) {
        (Scalar, Arithmetic | Comparison, Scalar) => {
            assert!(Comparison != op.kind() || bool_modifier);
            assert!(label_matching.is_none());
            assert!(group_modifier.is_none());
            Box::new(BinaryEvaluatorScalarScalar::new(op, left, right))
        }
        (Scalar, Arithmetic | Comparison, InstantVector) => {
            assert!(!bool_modifier || Comparison == op.kind());
            assert!(label_matching.is_none());
            assert!(group_modifier.is_none());
            Box::new(BinaryEvaluatorScalarVector::new(
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
            Box::new(BinaryEvaluatorVectorScalar::new(
                op,
                left,
                right,
                bool_modifier,
            ))
        }
        (InstantVector, Arithmetic | Comparison | Logical, InstantVector) => {
            assert!(!bool_modifier || Comparison == op.kind());
            assert!(group_modifier.is_none() || label_matching.is_some());
            Box::new(BinaryEvaluatorVectorVector::new(
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

/// BinaryEvaluatorScalarScalar
/// Ex:
///   1 + 2
///   42 / 6
///   2 ^ 64
///   ...
struct BinaryEvaluatorScalarScalar {
    op: BinaryOp,
    left: Box<dyn QueryValueIter>,
    right: Box<dyn QueryValueIter>,
}

impl BinaryEvaluatorScalarScalar {
    fn new(op: BinaryOp, left: Box<dyn QueryValueIter>, right: Box<dyn QueryValueIter>) -> Self {
        assert!(left.value_kind() == QueryValueKind::Scalar);
        assert!(right.value_kind() == QueryValueKind::Scalar);
        Self { op, left, right }
    }
}

impl std::iter::Iterator for BinaryEvaluatorScalarScalar {
    type Item = QueryValue;

    fn next(&mut self) -> Option<Self::Item> {
        use QueryValue::*;

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

impl QueryValueIter for BinaryEvaluatorScalarScalar {
    fn value_kind(&self) -> QueryValueKind {
        QueryValueKind::Scalar
    }
}

/// BinaryEvaluatorScalarVector
/// Ex:
///   2 * http_requests_total{}
///   42 - http_requests_total{method="GET"}
struct BinaryEvaluatorScalarVector {
    op: BinaryOp,
    left: Box<dyn QueryValueIter>,
    right: Box<dyn QueryValueIter>,
    bool_modifier: bool,
}

impl BinaryEvaluatorScalarVector {
    fn new(
        op: BinaryOp,
        left: Box<dyn QueryValueIter>,
        right: Box<dyn QueryValueIter>,
        bool_modifier: bool,
    ) -> Self {
        assert!(left.value_kind() == QueryValueKind::Scalar);
        assert!(right.value_kind() == QueryValueKind::InstantVector);
        Self {
            op,
            left,
            right,
            bool_modifier,
        }
    }
}

impl std::iter::Iterator for BinaryEvaluatorScalarVector {
    type Item = QueryValue;

    fn next(&mut self) -> Option<Self::Item> {
        use QueryValue::*;

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

        Some(InstantVector(rv.apply_scalar_op(
            |s| scalar_op_scalar_ex(self.op, lv, s, self.bool_modifier, s),
            self.op.kind() == BinaryOpKind::Comparison && !self.bool_modifier,
        )))
    }
}

impl QueryValueIter for BinaryEvaluatorScalarVector {
    fn value_kind(&self) -> QueryValueKind {
        QueryValueKind::InstantVector
    }
}

/// BinaryEvaluatorVectorScalar
/// Ex:
///   http_requests_total{} % 9000
///   http_requests_total{method="POST"} + 8
struct BinaryEvaluatorVectorScalar {
    op: BinaryOp,
    left: Box<dyn QueryValueIter>,
    right: Box<dyn QueryValueIter>,
    bool_modifier: bool,
}

impl BinaryEvaluatorVectorScalar {
    fn new(
        op: BinaryOp,
        left: Box<dyn QueryValueIter>,
        right: Box<dyn QueryValueIter>,
        bool_modifier: bool,
    ) -> Self {
        assert!(left.value_kind() == QueryValueKind::InstantVector);
        assert!(right.value_kind() == QueryValueKind::Scalar);
        Self {
            op,
            left,
            right,
            bool_modifier,
        }
    }
}

impl std::iter::Iterator for BinaryEvaluatorVectorScalar {
    type Item = QueryValue;

    fn next(&mut self) -> Option<Self::Item> {
        use QueryValue::*;

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

        Some(InstantVector(lv.apply_scalar_op(
            |s| scalar_op_scalar_ex(self.op, s, rv, self.bool_modifier, s),
            self.op.kind() == BinaryOpKind::Comparison && !self.bool_modifier,
        )))
    }
}

impl QueryValueIter for BinaryEvaluatorVectorScalar {
    fn value_kind(&self) -> QueryValueKind {
        QueryValueKind::InstantVector
    }
}

/// BinaryEvaluatorVectorVector
/// Ex:
///   http_requests_total{method="GET"} + http_requests_total{method="POST"}
///   http_requests_total{} + http_requests_content_length_bytes{} on (method, job)
///   http_requests_total{} + http_requests_content_length_bytes{} on (instance)
struct BinaryEvaluatorVectorVector {
    op: BinaryOp,
    left: std::iter::Peekable<Box<dyn QueryValueIter>>,
    right: std::iter::Peekable<Box<dyn QueryValueIter>>,
    bool_modifier: bool,
    label_matching: Option<LabelMatching>,
    group_modifier: Option<GroupModifier>,
}

impl BinaryEvaluatorVectorVector {
    fn new(
        op: BinaryOp,
        left: Box<dyn QueryValueIter>,
        right: Box<dyn QueryValueIter>,
        bool_modifier: bool,
        label_matching: Option<LabelMatching>,
        group_modifier: Option<GroupModifier>,
    ) -> Self {
        assert!(left.value_kind() == QueryValueKind::InstantVector);
        assert!(right.value_kind() == QueryValueKind::InstantVector);
        Self {
            op,
            left: left.peekable(),
            right: right.peekable(),
            bool_modifier,
            label_matching,
            group_modifier,
        }
    }
}

impl std::iter::Iterator for BinaryEvaluatorVectorVector {
    type Item = QueryValue;

    fn next(&mut self) -> Option<Self::Item> {
        use QueryValue::*;

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
            None => lv.apply_vector_op_one_to_one(
                |ls, rs| scalar_op_scalar_ex(self.op, ls, rs, self.bool_modifier, ls),
                &rv,
                self.label_matching.as_ref(),
                self.op.kind() == BinaryOpKind::Comparison && !self.bool_modifier,
            ),
            Some(GroupModifier::Left(ref include)) => lv.apply_vector_op_many_to_one(
                |ls, rs| scalar_op_scalar_ex(self.op, ls, rs, self.bool_modifier, ls),
                &rv,
                self.label_matching.as_ref(),
                include,
            ),
            Some(GroupModifier::Right(ref include)) => lv.apply_vector_op_one_to_many(
                |ls, rs| scalar_op_scalar_ex(self.op, ls, rs, self.bool_modifier, ls),
                &rv,
                self.label_matching.as_ref(),
                include,
            ),
        }))
    }
}

impl QueryValueIter for BinaryEvaluatorVectorVector {
    fn value_kind(&self) -> QueryValueKind {
        QueryValueKind::InstantVector
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
        Eql => ((lv - rv).abs() < f64::EPSILON) as i64 as SampleValue,
        Gte => (lv >= rv) as i64 as SampleValue,
        Gtr => (lv > rv) as i64 as SampleValue,
        Lss => (lv < rv) as i64 as SampleValue,
        Lte => (lv <= rv) as i64 as SampleValue,
        Neq => ((lv - rv).abs() > f64::EPSILON) as i64 as SampleValue,
        _ => unimplemented!(),
    }
}

fn scalar_op_scalar_ex(
    op: BinaryOp,
    lv: SampleValue,
    rv: SampleValue,
    bool_modifier: bool,
    comp_value: SampleValue,
) -> Option<SampleValue> {
    match (op.kind(), bool_modifier, scalar_op_scalar(op, lv, rv)) {
        (BinaryOpKind::Comparison, false, val) if val == 0.0 => None,
        (BinaryOpKind::Comparison, false, val) if (val - 1.0).abs() < f64::EPSILON => {
            Some(comp_value)
        }
        (_, _, val) => Some(val),
    }
}
