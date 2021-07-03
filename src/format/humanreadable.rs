use std::collections::BTreeMap;

use chrono::prelude::*;

use super::formatter::{Formatter, Value};
use crate::error::Result;
use crate::model::LabelsTrait;
use crate::parse::Entry;
use crate::query::{InstantVector, QueryValue};

pub struct HumanReadableFormatter {}

impl HumanReadableFormatter {
    pub fn new() -> Self {
        Self {}
    }

    // This is just a quick and dirty draft.
    fn encode_instant_vector(&self, vector: &InstantVector) -> Result<Vec<u8>> {
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
}

impl Formatter for HumanReadableFormatter {
    fn format(&self, value: &Value) -> Result<Vec<u8>> {
        match value {
            Value::Entry(Entry::Tuple(line_no, data)) => {
                Ok(format!("{}: {:?}", line_no, data).into_bytes())
            }
            Value::Entry(Entry::Dict(line_no, data)) => {
                Ok(format!("{}: {:?}", line_no, data).into_bytes())
            }
            Value::Record(record) => Ok(format!("{:?}", record).into_bytes()),
            Value::QueryValue(QueryValue::InstantVector(v)) => self.encode_instant_vector(v),
            _ => unimplemented!("coming soon..."),
        }
    }
}
