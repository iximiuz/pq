use std::collections::{BTreeMap, HashMap};

use serde_json;

use super::formatter::{Formatter, Value};
use super::promapi::PromApiFormatter;
use crate::error::Result;
use crate::parse::{Entry, Record};
use crate::query::{InstantVector, QueryValue, RangeVector};

pub struct JSONFormatter {
    verbose: bool,
    promfmt: PromApiFormatter,
}

impl JSONFormatter {
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            promfmt: PromApiFormatter::new(),
        }
    }

    fn format_tuple_entry(&self, line_no: usize, data: &[String]) -> Result<Vec<u8>> {
        if self.verbose {
            Ok(format!(
                "{}: {}",
                line_no,
                serde_json::to_string(data).map_err(|e| ("", e).into())?
            )
            .into_bytes())
        } else {
            serde_json::to_vec(data).map_err(|e| ("JSON serialization failed", e).into())
        }
    }

    fn format_dict_entry(&self, line_no: usize, data: &HashMap<String, String>) -> Result<Vec<u8>> {
        if self.verbose {
            Ok(format!("{}: {}", line_no, self.format_dict(data, "\t")).into_bytes())
        } else {
            Ok(self.format_dict(data, "\t").into_bytes())
        }
    }

    fn format_record(&self, record: &Record) -> Result<Vec<u8>> {
        let mut parts = Vec::new();
        if let Some(ts) = record.timestamp() {
            parts.push(ts.to_string_millis());
        }
        if record.labels().len() > 0 {
            parts.push(self.format_dict(record.labels(), "\t"));
        }
        if record.values().len() > 0 {
            parts.push(
                self.format_dict(
                    &record
                        .values()
                        .iter()
                        .map(|(key, val)| (key.clone(), val.to_string()))
                        .collect(),
                    "\t",
                ),
            );
        }

        if self.verbose {
            Ok(format!("{}: {}", record.line_no(), parts.join("\t")).into_bytes())
        } else {
            Ok(parts.join("\t").into_bytes())
        }
    }
}

impl Formatter for JSONFormatter {
    fn format(&self, value: &Value) -> Result<Vec<u8>> {
        match value {
            Value::Entry(Entry::Tuple(line_no, data)) => self.format_tuple_entry(*line_no, data),
            Value::Entry(Entry::Dict(line_no, data)) => self.format_dict_entry(*line_no, data),
            Value::Record(record) => self.format_record(record),
            Value::QueryValue(QueryValue::Scalar(n)) => Ok(n.to_string().into_bytes()),
            val => self.promfmt.format(val),
        }
    }
}
