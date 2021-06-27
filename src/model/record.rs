use std::collections::HashMap;

use super::labels::Labels;
use super::metric::MetricName;
use super::sample::SampleValue;
use super::timestamp::Timestamp;

pub type Values = HashMap<MetricName, SampleValue>;

#[derive(Debug)]
pub struct Record(pub Timestamp, pub Labels, pub Values);
