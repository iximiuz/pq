use std::collections::HashMap;

use crate::error::Result;
use crate::model::types::{Labels, Timestamp, Value};

pub type Values = HashMap<String, Value>;

#[derive(Debug)]
pub struct Record(pub Timestamp, pub Labels, pub Values);

pub trait Decoder {
    fn decode(&self, buf: &Vec<u8>) -> Result<Record>;
}
