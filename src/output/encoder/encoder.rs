use crate::error::Result;
use crate::query::ExprValue;

pub trait Encoder {
    fn encode(&self, value: &ExprValue) -> Result<Vec<u8>>;
}
