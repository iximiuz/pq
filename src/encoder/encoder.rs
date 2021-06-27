use crate::decoder::Entry;
use crate::error::Result;
use crate::model::Record;
use crate::query::ExprValue;

// Opposite to Entry
pub enum Outry {
    Entry(Entry, usize),
    Record(Record),
    Value(ExprValue),
}

pub trait Encoder {
    fn encode(&self, value: &Outry) -> Result<Vec<u8>>;
}
