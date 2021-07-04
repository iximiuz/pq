use super::record::Record;
use super::strategy::MappingStrategy;
use crate::error::Result;
use crate::model::Timestamp;
use crate::parse::Entry;
use crate::program::Mapper as MappingRules;
use crate::utils::time::TimeRange;

pub struct Mapper {
    entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
    strategy: MappingStrategy,
    range: TimeRange,
}

impl Mapper {
    pub fn new(
        entries: Box<dyn std::iter::Iterator<Item = Result<Entry>>>,
        mapping: MappingRules,
        range: Option<TimeRange>,
    ) -> Self {
        Self {
            entries,
            strategy: MappingStrategy::new(mapping),
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
                Some(Err(e)) => return Some(Err(e)),
                None => return None, // EOF
            };

            let record = match self.strategy.map(entry) {
                Ok(record) => record,
                Err(e) => return Some(Err(e)),
            };

            if record.timestamp().unwrap_or(Timestamp::MAX)
                < self.range.start().unwrap_or(Timestamp::MIN)
            {
                continue;
            }
            if record.timestamp().unwrap_or(Timestamp::MIN)
                > self.range.end().unwrap_or(Timestamp::MAX)
            {
                continue;
            }

            return Some(Ok(record));
        }
    }
}
