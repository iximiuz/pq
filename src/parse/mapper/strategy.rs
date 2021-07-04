use chrono::prelude::*;

use super::record::{Record, Values};
use crate::error::{Error, Result};
use crate::model::{Labels, SampleValue};
use crate::parse::Entry;
use crate::program::{FieldLoc, FieldType, Mapper as MappingRules, MapperField};

pub struct MappingStrategy {
    mapping: MappingRules,
}

impl MappingStrategy {
    pub fn new(mapping: MappingRules) -> Self {
        Self { mapping }
    }

    pub fn map(&self, entry: Entry) -> Result<Record> {
        let mut timestamp = None;
        let mut values = Values::new();
        let mut labels = Labels::new();

        for field in self.mapping.fields.iter() {
            if let FieldType::Const(ref value) = field.typ {
                labels.insert(field.end_name(), value.clone());
                continue;
            }

            let datum = get_entry_field(&entry, field)?;

            match &field.typ {
                FieldType::Auto => {
                    if let Ok(n) = datum.parse::<SampleValue>() {
                        values.insert(field.end_name(), n);
                    } else {
                        labels.insert(field.end_name(), datum);
                    }
                    // TODO: else if Ok(ts) = datum.parse::<Timestamp>() { ... }
                }
                FieldType::Number => {
                    if let Ok(n) = datum.parse::<SampleValue>() {
                        values.insert(field.end_name(), n);
                    }
                    return Err(Error::new("could not parse numeric field"));
                }
                FieldType::String => {
                    labels.insert(field.end_name(), datum);
                }
                FieldType::Timestamp(format) => {
                    timestamp =
                        Some(parse_timestamp(&datum, format.as_deref())?.timestamp_millis());
                }
                _ => unreachable!(),
            }
        }

        Ok(Record::new(entry.line_no(), timestamp, labels, values))
    }
}

// fn decode(&self, buf: &Vec<u8>) -> Result<Entry> {
//     let record_caps = self.re.captures(buf).ok_or("no match found")?;
//
//     let timestamp = parse_record_timestamp(
//         &String::from_utf8(
//             record_caps
//                 .get(self.timestamp_cap.pos + 1)
//                 .ok_or("timestamp capture is empty")?
//                 .as_bytes()
//                 .to_vec(),
//         )
//         .map_err(|e| ("couldn't decode UTF-8 timestamp value", e))?,
//         Some(&self.timestamp_cap.format),
//     )?;
//
//     let mut metrics = Values::new();
//     for metric_cap in self.metric_caps.iter() {
//         if let Some(metric) = record_caps.get(metric_cap.pos + 1) {
//             metrics.insert(
//                 metric_cap.name.clone(),
//                 String::from_utf8(metric.as_bytes().to_vec())
//                     .map_err(|e| ("couldn't decode UTF-8 metric value", e))?
//                     .parse::<f64>()
//                     .map_err(|e| ("couldn't parse metric value into f64", e))?,
//             );
//         }
//     }
//
//     if metrics.len() == 0 {
//         return Err(Error::new("no metric match found"));
//     }
//
//     let mut labels = HashMap::new();
//     for label_cap in self.label_caps.iter() {
//         if let Some(label) = record_caps.get(label_cap.pos + 1) {
//             labels.insert(
//                 label_cap.name.clone(),
//                 String::from_utf8(label.as_bytes().to_vec())
//                     .map_err(|e| ("couldn't decode UTF-8 label value", e))?,
//             );
//         }
//     }
//
//     Ok(Record(timestamp.timestamp_millis(), labels, metrics))
// }

fn get_entry_field(entry: &Entry, field: &MapperField) -> Result<String> {
    match (entry, &field.loc) {
        (Entry::Tuple(_, tuple), FieldLoc::Position(idx)) => {
            if *idx > tuple.len() {
                Err(Error::new("tuple entry index out of range"))
            } else {
                Ok(tuple[*idx].clone())
            }
        }
        (Entry::Dict(_, dict), FieldLoc::Name(name)) => {
            if let Some(datum) = dict.get(name) {
                Ok(datum.clone())
            } else {
                Err(Error::new("dict entry field not found"))
            }
        }
        (Entry::Tuple(_, _), FieldLoc::Name(_)) => {
            Err(Error::new("tuple entry cannot be mapped with named fields"))
        }
        (Entry::Dict(_, _), FieldLoc::Position(_)) => Err(Error::new(
            "dict entry cannot be mapped with positional fields",
        )),
    }
}

fn parse_timestamp(timestamp: &str, format: Option<&str>) -> Result<DateTime<Utc>> {
    match format {
        Some(f) => Utc.datetime_from_str(timestamp, f),
        None => timestamp.parse::<DateTime<Utc>>(),
    }
    .map_err(|e| (Error::from(("couldn't parse timestamp", e))))
}
