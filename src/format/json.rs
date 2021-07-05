use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use serde_json;

use super::formatter::{Formatter, Value};
use super::promapi::PromApiFormatter;
use crate::error::Result;
use crate::model::Timestamp;
use crate::parse::{Entry, Record};
use crate::query::QueryValue;

#[derive(Serialize)]
struct TupleEntryRepr<'a> {
    line: usize,
    data: &'a [String],
}

#[derive(Serialize)]
struct DictEntryRepr<'a> {
    line: usize,
    data: BTreeMap<&'a String, &'a String>,
}

#[derive(Serialize)]
struct RecordRepr<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<Timestamp>,
    labels: BTreeMap<&'a String, &'a String>,
    values: BTreeMap<&'a String, &'a f64>,
}

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

    fn format_tuple_entry(&self, line: usize, data: &[String]) -> Result<Vec<u8>> {
        if self.verbose {
            serde_json::to_vec(&TupleEntryRepr { line, data })
        } else {
            serde_json::to_vec(data)
        }
        .map_err(|e| ("JSON serialization failed", e).into())
    }

    fn format_dict_entry(&self, line: usize, data: &HashMap<String, String>) -> Result<Vec<u8>> {
        if self.verbose {
            serde_json::to_vec(&DictEntryRepr {
                line,
                data: data.iter().collect(),
            })
        } else {
            serde_json::to_vec(data)
        }
        .map_err(|e| ("JSON serialization failed", e).into())
    }

    fn format_record(&self, record: &Record) -> Result<Vec<u8>> {
        let mut repr = RecordRepr {
            line: None,
            timestamp: record.timestamp(),
            labels: record.labels().iter().collect(),
            values: record.values().iter().collect(),
        };

        if self.verbose {
            repr.line = Some(record.line_no());
        }

        serde_json::to_vec(&repr).map_err(|e| ("JSON serialization failed", e).into())
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
