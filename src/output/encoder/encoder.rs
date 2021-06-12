use crate::engine::ExprValue;
use crate::error::Result;

pub trait Encoder {
    fn encode(&self, value: &ExprValue) -> Result<Vec<u8>>;
}
