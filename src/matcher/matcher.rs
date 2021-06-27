use crate::decoder::Entry;
use crate::error::Result;
use crate::model::Record;

pub trait Matcher {
    fn match_record(entry: &Entry) -> Result<Record>;
}
