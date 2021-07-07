use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::prelude::*;

use super::formatter::{Formatter, Value};
use crate::error::Result;
use crate::model::{LabelsTrait, SampleValue, TimestampTrait};
use crate::parse::{Entry, Record};
use crate::query::{InstantVector, QueryValue, RangeVector};

pub struct HumanReadableFormatter {
    verbose: bool,
    interactive: bool,
}

impl HumanReadableFormatter {
    pub fn new(verbose: bool, interactive: bool) -> Self {
        Self {
            verbose,
            interactive,
        }
    }

    fn format_tuple_entry(&self, line_no: usize, data: &[String]) -> Result<Vec<u8>> {
        if self.verbose {
            Ok(format!("{}: {}", line_no, data.join("\t")).into_bytes())
        } else {
            Ok(data.join("\t").into_bytes())
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

    fn format_instant_vector(&self, vector: &InstantVector) -> Result<Vec<u8>> {
        let mut lines = Vec::new();

        for (labels, value) in vector.samples() {
            let mut parts = vec![format!("{}\t", vector.timestamp().to_string_millis())];

            if let Some(metric) = labels.name() {
                parts.push(format!("{}", metric));
            }

            let labels = labels.without(&HashSet::new()); // to drop __name__
            if labels.len() > 0 || labels.name().is_some() {
                parts.push(format!("{{{}}}\t\t\t", self.format_dict(&labels, ", ")));
            }

            parts.push(value.to_string());

            lines.push(parts.join(""));
        }

        if lines.len() == 0 {
            if !self.verbose {
                return Ok(Vec::new());
            }

            lines.push(format!(
                "{} <no data>",
                vector.timestamp().to_string_millis()
            ));
        }

        Ok(String::into_bytes(lines.join("\n")))
    }

    // This is just a quick and dirty draft.
    fn format_instant_vector_interactive(&self, vector: &InstantVector) -> Result<Vec<u8>> {
        let ts = NaiveDateTime::from_timestamp(vector.timestamp() / 1000, 0);
        let mut lines = vec![
            // format!("{}[2J", 27 as char),
            format!("{esc}[2J{esc}[1;1H", esc = 27 as char),
            ts.format("%Y-%m-%d %H:%M:%S").to_string(),
            "-".to_string(),
        ];

        let mut prefix = "";
        for (labels, value) in vector.samples() {
            if let Some(metric) = labels.name() {
                lines.push(metric.clone());
                lines.push("\n".to_string());
                prefix = "\t";
            }

            let mut line = vec![];
            for (label_name, label_value) in labels.iter().collect::<BTreeMap<_, _>>() {
                line.push(format!("{}{}: '{}'", prefix, label_name, label_value));
            }
            line.push(format!("\t\t\t{}", value));

            lines.push(line.join("\t\t"));
        }

        Ok(String::into_bytes(lines.join("\n")))
    }

    fn format_range_vector(&self, vector: &RangeVector) -> Result<Vec<u8>> {
        let mut lines = Vec::new();

        for (labels, values) in vector.samples() {
            let mut parts = vec![format!("{}\t", vector.timestamp().to_string_millis())];

            if let Some(metric) = labels.name() {
                parts.push(format!("{}", metric));
            }

            let labels = labels.without(&HashSet::new()); // to drop __name__
            if labels.len() > 0 || labels.name().is_some() {
                parts.push(format!("{{{}}}\t\t\t", self.format_dict(&labels, ", ")));
            }

            lines.push(parts.join(""));
            for (val, ts) in values.iter().rev() {
                lines.push(format!("\t{} @ {}", val, ts.to_string_millis()));
            }
        }

        if lines.len() == 0 {
            if !self.verbose {
                return Ok(Vec::new());
            }

            lines.push(format!(
                "{} <no data>",
                vector.timestamp().to_string_millis()
            ));
        }

        Ok(String::into_bytes(lines.join("\n")))
    }

    fn format_scalar(&self, n: SampleValue) -> Result<Vec<u8>> {
        Ok(n.to_string().into_bytes())
    }

    fn format_dict(&self, dict: &HashMap<String, String>, sep: &str) -> String {
        let ordered = dict.iter().collect::<BTreeMap<_, _>>();
        ordered
            .iter()
            .map(|(key, val)| format!("{}={}", key, val))
            .collect::<Vec<_>>()
            .join(sep)
    }
}

impl Formatter for HumanReadableFormatter {
    fn format(&self, value: &Value) -> Result<Vec<u8>> {
        if self.interactive {
            return match value {
                Value::QueryValue(QueryValue::InstantVector(v)) => {
                    self.format_instant_vector_interactive(v)
                }
                _ => unimplemented!("interactive mode is not supported for this type of result"),
            };
        }

        match value {
            Value::Entry(Entry::Tuple(line_no, data)) => self.format_tuple_entry(*line_no, data),
            Value::Entry(Entry::Dict(line_no, data)) => self.format_dict_entry(*line_no, data),
            Value::Record(record) => self.format_record(record),
            Value::QueryValue(QueryValue::InstantVector(v)) => self.format_instant_vector(v),
            Value::QueryValue(QueryValue::RangeVector(v)) => self.format_range_vector(v),
            Value::QueryValue(QueryValue::Scalar(n)) => self.format_scalar(*n),
        }
    }
}
