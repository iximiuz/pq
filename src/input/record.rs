use std::collections::HashMap;

use super::entry::Entry;
use super::matcher::RecordMatcher;
use crate::error::Result;
use crate::model::{Labels, MetricName, SampleValue, Timestamp};

pub type Values = HashMap<MetricName, SampleValue>;

#[derive(Debug)]
pub struct Record(pub Timestamp, pub Labels, pub Values);

pub struct RecordReader {
    entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
    matcher: Box<dyn RecordMatcher>,
}

impl RecordReader {
    pub fn new(
        entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
        matcher: Box<dyn RecordMatcher>,
    ) -> Self {
        Self { entries, matcher }
    }
}

impl std::iter::Iterator for RecordReader {
    type Item = Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = match self.entries.next() {
                Some(Ok(entry)) => (entry),
                Some(Err(e)) => {
                    return Some(Err(("reader failed", e).into()));
                }
                None => return None, // EOF
            };

            // TODO:
            // Tiny hack...
            // values.insert("__line__".to_owned(), self.line_no as SampleValue);

            // if sample.timestamp() > self.last_instant.unwrap_or(Timestamp::MAX) {
            //     // Input not really drained, but we've seen enough.
            //     return None;
            // }

            return None;
        }
    }
}
