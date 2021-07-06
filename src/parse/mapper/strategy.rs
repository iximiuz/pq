use super::record::{Record, Values};
use crate::error::{Error, Result};
use crate::model::{Labels, SampleValue, Timestamp};
use crate::parse::Entry;
use crate::program::{FieldLoc, FieldType, Mapper as MappingRules, MapperField};
use crate::utils::time::{parse_time, try_parse_time};

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
                    } else {
                        return Err(Error::new("could not parse numeric field"));
                    }
                }
                FieldType::String => {
                    labels.insert(field.end_name(), datum);
                }
                FieldType::Timestamp(format) => {
                    timestamp = Some(parse_timestamp_field(&datum, format.as_deref())?);
                }
                _ => unreachable!(),
            }
        }

        Ok(Record::new(entry.line_no(), timestamp, labels, values))
    }
}

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

fn parse_timestamp_field(timestamp: &str, format: Option<&str>) -> Result<Timestamp> {
    match format {
        Some(format) => parse_time(timestamp, format),
        None => match try_parse_time(timestamp) {
            Some(timestamp) => Ok(timestamp),
            None => return Err(Error::new("couldn't guess time format")),
        },
    }
}
