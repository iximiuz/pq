mod arithmetic;

use super::value::{ExprValueIter, ExprValueKind};
use crate::parser::ast::{BinaryOp, BinaryOpKind, GroupModifier, VectorMatching};
use arithmetic::{
    ArithmeticExprExecutorScalarScalar, ArithmeticExprExecutorScalarVector,
    ArithmeticExprExecutorVectorScalar, ArithmeticExprExecutorVectorVector,
};

pub(super) fn create_binary_expr_executor(
    op: BinaryOp,
    left: Box<dyn ExprValueIter>,
    right: Box<dyn ExprValueIter>,
    bool_modifier: bool,
    vector_matching: Option<VectorMatching>,
    group_modifier: Option<GroupModifier>,
) -> Box<dyn ExprValueIter> {
    use BinaryOpKind::*;
    use ExprValueKind::*;

    match (left.value_kind(), op.kind(), right.value_kind()) {
        (Scalar, Arithmetic, Scalar) => {
            assert!(!bool_modifier);
            assert!(vector_matching.is_none());
            assert!(group_modifier.is_none());
            Box::new(ArithmeticExprExecutorScalarScalar::new(op, left, right))
        }
        // (Scalar, Comparison, Scalar) => ()
        (Scalar, Arithmetic, InstantVector) => {
            assert!(!bool_modifier);
            assert!(vector_matching.is_none());
            assert!(group_modifier.is_none());
            Box::new(ArithmeticExprExecutorScalarVector::new(op, left, right))
        }
        // (Scalar, Comparison, InstantVector) => ()
        (InstantVector, Arithmetic, Scalar) => {
            assert!(!bool_modifier);
            assert!(vector_matching.is_none());
            assert!(group_modifier.is_none());
            Box::new(ArithmeticExprExecutorVectorScalar::new(op, left, right))
        }
        // (InstantVector, Comparison, Scalar) => ()
        (InstantVector, Arithmetic, InstantVector) => {
            assert!(!bool_modifier);
            Box::new(ArithmeticExprExecutorVectorVector::new(op, left, right))
        }
        // (InstantVector, Comparison, InstantVector) => ()
        // (InstantVector, Logical, InstantVector) => ()
        (lk, ok, rk) => unimplemented!("{:?} {:?} {:?} operation is not supported", lk, ok, rk),
    }
}
