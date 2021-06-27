use crate::decoder::Entry;
use crate::error::Result;
use crate::model::Record;

pub trait Matcher {
    fn match_record(&self, entry: &Entry) -> Result<Record>;
}
