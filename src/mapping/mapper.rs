use crate::error::Result;
use crate::model::RecordValue;

pub trait Mapper {
    fn map(&self, value: &RecordValue) -> Result<Option<RecordValue>>;
}

//  use super::super::parser::{FieldLoc, FieldType, Mapper as MappingRules, MapperField};
//  use super::record::{Record, Values};
//  use crate::error::{Error, Result};
//  use crate::model::{Labels, SampleValue, Timestamp};
//  use crate::parse::Entry;
//  use crate::utils::time::{parse_time, try_parse_time, TimeRange};

// pub struct Mapper {
//     entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
//     mapping: MappingRules,
//     range: TimeRange,
// }
//
// impl Mapper {
//     pub fn new(
//         entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
//         mapping: MappingRules,
//         range: Option<TimeRange>,
//     ) -> Self {
//         Self {
//             entries,
//             mapping,
//             range: range.unwrap_or_else(TimeRange::infinity),
//         }
//     }
//
//     fn map(&self, entry: Entry) -> Result<Record> {
//         let mut timestamp = None;
//         let mut values = Values::new();
//         let mut labels = Labels::new();
//
//         for field in self.mapping.fields.iter() {
//             if let FieldType::Const(ref value) = field.typ {
//                 labels.insert(field.end_name(), value.clone());
//                 continue;
//             }
//
//             let datum = get_entry_field(&entry, field)?;
//
//             match &field.typ {
//                 FieldType::Auto => {
//                     if let Ok(n) = datum.parse::<SampleValue>() {
//                         values.insert(field.end_name(), n);
//                     } else {
//                         labels.insert(field.end_name(), datum);
//                     }
//                     // TODO: else if Ok(ts) = datum.parse::<Timestamp>() { ... }
//                 }
//                 FieldType::Number => {
//                     if let Ok(n) = datum.parse::<SampleValue>() {
//                         values.insert(field.end_name(), n);
//                     } else {
//                         return Err(Error::new("could not parse numeric field"));
//                     }
//                 }
//                 FieldType::String => {
//                     labels.insert(field.end_name(), datum);
//                 }
//                 FieldType::Timestamp(format) => {
//                     timestamp = Some(parse_timestamp_field(&datum, format.as_deref())?);
//                 }
//                 _ => unreachable!(),
//             }
//         }
//
//         Ok(Record::new(entry.line_no(), timestamp, labels, values))
//     }
// }
//
// impl std::iter::Iterator for Mapper {
//     type Item = Result<Record>;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         loop {
//             let entry = match self.entries.next() {
//                 Some(Ok(entry)) => entry,
//                 Some(Err(e)) => return Some(Err(e)),
//                 None => return None, // EOF
//             };
//
//             let record = match self.strategy.map(entry) {
//                 Ok(record) => record,
//                 Err(e) => return Some(Err(e)),
//             };
//
//             if record.timestamp().unwrap_or(Timestamp::MAX)
//                 < self.range.start().unwrap_or(Timestamp::MIN)
//             {
//                 continue;
//             }
//             if record.timestamp().unwrap_or(Timestamp::MIN)
//                 > self.range.end().unwrap_or(Timestamp::MAX)
//             {
//                 return None; // not a EOF but we are out of requested range.
//             }
//
//             return Some(Ok(record));
//         }
//     }
// }
//
// // TODO: make it a method of Entry.
// fn get_entry_field(entry: &Entry, field: &MapperField) -> Result<String> {
//     match (entry, &field.loc) {
//         (Entry::Tuple(_, tuple), FieldLoc::Position(idx)) => {
//             if *idx > tuple.len() {
//                 Err(Error::new("tuple entry index out of range"))
//             } else {
//                 Ok(tuple[*idx].clone())
//             }
//         }
//         (Entry::Dict(_, dict), FieldLoc::Name(name)) => {
//             if let Some(datum) = dict.get(name) {
//                 Ok(datum.clone())
//             } else {
//                 Err(Error::new("dict entry field not found"))
//             }
//         }
//         (Entry::Tuple(_, _), FieldLoc::Name(_)) => {
//             Err(Error::new("tuple entry cannot be mapped with named fields"))
//         }
//         (Entry::Dict(_, _), FieldLoc::Position(_)) => Err(Error::new(
//             "dict entry cannot be mapped with positional fields",
//         )),
//     }
// }
//
// fn parse_timestamp_field(timestamp: &str, format: Option<&str>) -> Result<Timestamp> {
//     match format {
//         Some(format) => parse_time(timestamp, format),
//         None => match try_parse_time(timestamp) {
//             Some(timestamp) => Ok(timestamp),
//             None => Err(Error::new("couldn't guess time format")),
//         },
//     }
// }
