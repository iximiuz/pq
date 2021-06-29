use crate::error::Result;
use crate::input::{Entry, Record};
use crate::query::ExprValue;

pub enum Encodable {
    Entry(Entry),
    Record(Record),
    ExprValue(ExprValue),
}

pub trait Encoder {
    fn encode(&self, value: &Encodable) -> Result<Vec<u8>>;
}
