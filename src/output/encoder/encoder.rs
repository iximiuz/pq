use crate::error::Result;
use crate::engine::Value;

pub trait Encoder {
    fn encode(&self, value: &Value) -> Result<Vec<u8>>;
}
