use crate::error::Result;
use crate::parse::{Entry, Record};
use crate::query::QueryValue;

pub enum Value {
    Entry(Entry),
    Record(Record),
    QueryValue(QueryValue),
}

pub trait Encoder {
    fn encode(&self, value: &Value) -> Result<Vec<u8>>;
}
