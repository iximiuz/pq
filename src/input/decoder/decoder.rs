use std::collections::HashMap;

use crate::error::Result;
use crate::model::types::Timestamp;

type Labels = HashMap<String, String>;
type Values = HashMap<String, f64>;

#[derive(Debug)]
pub struct Record(pub Timestamp, pub Labels, pub Values);

pub trait Decoder {
    fn decode(&mut self, buf: &Vec<u8>) -> Result<Record>;
}
