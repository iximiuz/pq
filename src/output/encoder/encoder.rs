use crate::error::Result;
use crate::input::{Entry, Record};
use crate::query::QueryValue;

pub enum Encodable {
    Entry(Entry),
    Record(Record),
    QueryValue(QueryValue),
}

pub trait Encoder {
    fn encode(&self, value: &Encodable) -> Result<Vec<u8>>;
}
