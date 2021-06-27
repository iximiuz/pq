use super::labels::Labels;
use super::metric::MetricName;
use super::timestamp::Timestamp;

pub type SampleValue = f64;

#[derive(Debug)]
pub struct Sample {
    value: SampleValue,
    timestamp: Timestamp,
    labels: Labels,
}

impl Sample {
    pub fn new(
        name: MetricName,
        value: SampleValue,
        timestamp: Timestamp,
        mut labels: Labels,
    ) -> Self {
        labels.insert("__name__".into(), name);
        Self {
            value,
            timestamp,
            labels,
        }
    }

    #[inline]
    pub fn value(&self) -> SampleValue {
        self.value
    }

    #[inline]
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    #[inline]
    pub fn labels(&self) -> &Labels {
        &self.labels
    }

    pub fn label(&self, name: &str) -> Option<&MetricName> {
        self.labels.get(name)
    }
}
