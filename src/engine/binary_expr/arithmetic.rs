use super::super::value::{ExprValue, ExprValueIter, ExprValueKind, InstantVector as Vector};
use crate::model::types::SampleValue;
use crate::parser::ast::{BinaryOp, BinaryOpKind};

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
        use BinaryOp::*;
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

        Some(Scalar(match self.op {
            Add => lv + rv,
            Div => lv / rv,
            Mul => lv * rv,
            Mod => lv % rv,
            Pow => SampleValue::powf(lv, rv),
            Sub => lv - rv,
            _ => unimplemented!(),
        }))
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
        use BinaryOp::*;
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

        match self.op {
            Add => rv.mutate_values(|(_, val)| *val = lv + *val),
            Div => rv.mutate_values(|(_, val)| *val = lv / *val),
            Mul => rv.mutate_values(|(_, val)| *val = lv * *val),
            Mod => rv.mutate_values(|(_, val)| *val = lv % *val),
            Pow => rv.mutate_values(|(_, val)| *val = SampleValue::powf(lv, *val)),
            Sub => rv.mutate_values(|(_, val)| *val = lv - *val),
            _ => unimplemented!(),
        }
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
        use BinaryOp::*;
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

        match self.op {
            Add => lv.mutate_values(|(_, val)| *val = *val + rv),
            Div => lv.mutate_values(|(_, val)| *val = *val / rv),
            Mul => lv.mutate_values(|(_, val)| *val = *val * rv),
            Mod => lv.mutate_values(|(_, val)| *val = *val % rv),
            Pow => lv.mutate_values(|(_, val)| *val = SampleValue::powf(*val, rv)),
            Sub => lv.mutate_values(|(_, val)| *val = *val - rv),
            _ => unimplemented!(),
        }

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
}

impl ArithmeticExprExecutorVectorVector {
    pub fn new(op: BinaryOp, left: Box<dyn ExprValueIter>, right: Box<dyn ExprValueIter>) -> Self {
        assert!(op.kind() == BinaryOpKind::Arithmetic);
        assert!(left.value_kind() == ExprValueKind::InstantVector);
        assert!(right.value_kind() == ExprValueKind::InstantVector);
        Self {
            op,
            left: left.peekable(),
            right: right.peekable(),
        }
    }
}

impl std::iter::Iterator for ArithmeticExprExecutorVectorVector {
    type Item = ExprValue;

    fn next(&mut self) -> Option<Self::Item> {
        use BinaryOp::*;
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

        Some(InstantVector(lv.match_vector(
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

impl ExprValueIter for ArithmeticExprExecutorVectorVector {
    fn value_kind(&self) -> ExprValueKind {
        ExprValueKind::InstantVector
    }
}
