use super::super::super::entry::Entry;
use super::super::record::Record;
use crate::error::Result;

pub trait RecordMatcher {
    fn match_record(&self, entry: &Entry) -> Result<Record>;
}
