use std::collections::HashMap;

use crate::model::{Labels, MetricName, SampleValue, Timestamp};

pub type Values = HashMap<MetricName, SampleValue>;

#[derive(Debug)]
pub struct Record {
    line_no: usize,
    timestamp: Option<Timestamp>,
    labels: Labels,
    values: Values,
}

impl Record {
    pub(super) fn new(
        line_no: usize,
        timestamp: Option<Timestamp>,
        labels: Labels,
        values: Values,
    ) -> Self {
        Self {
            line_no,
            timestamp,
            labels,
            values,
        }
    }

    #[inline]
    pub fn line_no(&self) -> usize {
        self.line_no
    }

    #[inline]
    pub fn timestamp(&self) -> Option<Timestamp> {
        self.timestamp
    }

    #[inline]
    pub fn labels(&self) -> &Labels {
        &self.labels
    }

    #[inline]
    pub fn values(&self) -> &Values {
        &self.values
    }
}
