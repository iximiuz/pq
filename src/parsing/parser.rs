use crate::error::Result;
use crate::model::RecordValue;

pub trait Parser {
    fn parse(&self, buf: &[u8]) -> Result<Option<RecordValue>>;
}
