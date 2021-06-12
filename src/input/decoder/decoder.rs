use std::collections::HashMap;

use crate::error::Result;
use crate::model::types::{Labels, MetricName, SampleValue, Timestamp};

pub type Values = HashMap<MetricName, SampleValue>;

#[derive(Debug)]
pub struct Record(pub Timestamp, pub Labels, pub Values);

pub trait Decoder {
    fn decode(&self, buf: &Vec<u8>) -> Result<Record>;
}
