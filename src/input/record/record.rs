use std::collections::HashMap;

use super::super::entry::Entry;
use super::matcher::RecordMatcher;
use crate::error::Result;
use crate::model::{Labels, MetricName, SampleValue, Timestamp};
use crate::utils::time::TimeRange;

pub type Values = HashMap<MetricName, SampleValue>;

#[derive(Debug)]
pub struct Record(pub Timestamp, pub Labels, pub Values);

pub struct RecordReader {
    entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
    matcher: Box<dyn RecordMatcher>,
    range: TimeRange,
}

impl RecordReader {
    pub fn new(
        entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
        matcher: Box<dyn RecordMatcher>,
        range: Option<TimeRange>,
    ) -> Self {
        Self {
            entries,
            matcher,
            range: range.unwrap_or(TimeRange::infinity()),
        }
    }
}

impl std::iter::Iterator for RecordReader {
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

            let (ts, labels, mut values) = match self.matcher.match_record(&entry) {
                Ok(Record(ts, ls, vs)) => (ts, ls, vs),
                Err(_) => {
                    // TODO: eprintln!() if verbose
                    continue;
                }
            };

            // Tiny hack...
            values.insert("__line__".to_owned(), entry.0 as SampleValue);

            if ts < self.range.start().unwrap_or(Timestamp::MIN) {
                continue;
            }
            if ts > self.range.end().unwrap_or(Timestamp::MAX) {
                continue;
            }

            return Some(Ok(Record(ts, labels, values)));
        }
    }
}
