use std::collections::HashMap;

use crate::error::Result;
use crate::model::{Labels, MetricName, SampleValue, Timestamp};
use crate::parse::Entry;
use crate::program::Mapper as MapperOpts;
use crate::utils::time::TimeRange;

pub type Values = HashMap<MetricName, SampleValue>;

#[derive(Debug)]
pub struct Record {
    line_no: usize,
    timestamp: Option<Timestamp>,
    labels: Labels,
    values: Values,
}

impl Record {
    #[inline]
    pub fn line_no(&self) -> usize {
        self.line_no
    }

    #[inline]
    pub fn timestamp(&self) -> Option<Timestamp> {
        self.timestamp
    }

    #[inline]
    pub fn labels(&self) -> &Labels {
        &self.labels
    }

    #[inline]
    pub fn values(&self) -> &Values {
        &self.values
    }
}

pub struct Mapper {
    entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
    opts: MapperOpts,
    range: TimeRange,
}

impl Mapper {
    pub fn new(
        entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
        opts: MapperOpts,
        range: Option<TimeRange>,
    ) -> Self {
        Self {
            entries,
            opts,
            range: range.unwrap_or(TimeRange::infinity()),
        }
    }
}

impl std::iter::Iterator for Mapper {
    type Item = Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = match self.entries.next() {
            Some(Ok(entry)) => entry,
            Some(Err(e)) => return Some(Err(e)),
            None => return None, // EOF
        };

        // let record = match self.matcher.as_ref().unwrap().match_record(&entry) {
        //     Ok(record) => record,
        //     Err(_) => {
        //         // TODO: eprintln!() if verbose
        //         continue;
        //     }
        // };

        // if record.1 < self.range.start().unwrap_or(Timestamp::MIN) {
        //     continue;
        // }
        // if record.1 > self.range.end().unwrap_or(Timestamp::MAX) {
        //     continue;
        // }

        // return Some(Ok(record));
        return None;
    }
}
