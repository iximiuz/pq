use std::collections::HashMap;

use super::timestamp::Timestamp;

pub struct Record {
    line_no: usize,
    value: RecordValue,
    // timestamp_locator: Option<Locator>
}

impl Record {
    pub fn new(line_no: usize, value: RecordValue) -> Self {
        Self { line_no, value }
    }

    #[inline]
    pub fn line_no(&self) -> usize {
        self.line_no
    }
}

// pub type Values = HashMap<MetricName, SampleValue>;

pub enum RecordValue {
    Number(f64),
    String(String),
    Timestamp(Timestamp),
    Array(Vec<RecordValue>),
    Object(HashMap<String, RecordValue>),
}
