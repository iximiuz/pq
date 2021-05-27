use std::rc::Rc;

use crate::input::Sample;

// Every Expr can be evaluated to a Value.
#[derive(Debug)]
pub(super) enum Value {
    InstantVector(Vec<Rc<Sample>>),
    RangeVector(Vec<Vec<Rc<Sample>>>),
    Scalar(f64),
}
