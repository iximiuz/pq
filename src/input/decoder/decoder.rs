use crate::error::Result;

use super::record::Record;

pub trait Decoder {
    fn decode(&mut self, buf: &mut Vec<u8>) -> Result<Record>;
}
