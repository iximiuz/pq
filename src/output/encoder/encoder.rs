use crate::engine::ValueKind;
use crate::error::Result;

pub trait Encoder {
    fn encode(&self, value: &ValueKind) -> Result<Vec<u8>>;
}
