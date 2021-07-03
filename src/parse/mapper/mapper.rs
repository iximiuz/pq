use std::collections::HashMap;

use crate::error::Result;
use crate::model::{Labels, MetricName, SampleValue, Timestamp};
use crate::parse::Entry;
use crate::utils::time::TimeRange;

pub type Values = HashMap<MetricName, SampleValue>;

#[derive(Debug)]
pub struct Record(pub usize, pub Timestamp, pub Labels, pub Values);

pub trait RecordMatcher {
    fn match_record(&self, entry: &Entry) -> Result<Record>;
}

pub struct Mapper {
    entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
    matcher: Option<Box<dyn RecordMatcher>>,
    range: TimeRange,
}

impl Mapper {
    pub fn new(
        entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
        range: Option<TimeRange>,
    ) -> Self {
        Self {
            entries,
            matcher: None,
            range: range.unwrap_or(TimeRange::infinity()),
        }
    }
}

impl std::iter::Iterator for Mapper {
    type Item = Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = match self.entries.next() {
                Some(Ok(entry)) => entry,
                Some(Err(e)) => {
                    return Some(Err(("reader failed", e).into()));
                }
                None => return None, // EOF
            };

            let record = match self.matcher.as_ref().unwrap().match_record(&entry) {
                Ok(record) => record,
                Err(_) => {
                    // TODO: eprintln!() if verbose
                    continue;
                }
            };

            if record.1 < self.range.start().unwrap_or(Timestamp::MIN) {
                continue;
            }
            if record.1 > self.range.end().unwrap_or(Timestamp::MAX) {
                continue;
            }

            return Some(Ok(record));
        }
    }
}
