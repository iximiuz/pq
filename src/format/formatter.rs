use crate::error::Result;
use crate::parse::{Entry, Record};
use crate::query::QueryValue;

pub enum Value {
    Entry(Entry),
    Record(Record),
    QueryValue(QueryValue),
}

pub trait Formatter {
    fn format(&self, value: &Value) -> Result<Vec<u8>>;
}
