use super::formatter::{Formatter, Value};
use crate::error::Result;
use crate::parse::Entry;

pub struct JSONFormatter {}

impl JSONFormatter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Formatter for JSONFormatter {
    fn format(&self, value: &Value) -> Result<Vec<u8>> {
        match value {
            Value::Entry(Entry::Tuple(line_no, data)) => {
                Ok(format!("KINDA JSON {}: {:?}", line_no, data).into_bytes())
            }
            Value::Entry(Entry::Dict(line_no, data)) => {
                Ok(format!("KINDA JSON {}: {:?}", line_no, data).into_bytes())
            }
            _ => unimplemented!("coming soon..."),
        }
    }
}
