use std::collections::HashMap;

type Timestamp = i64;

#[derive(Debug)]
pub struct Record {
    timestamp: Timestamp,
    labels: HashMap<String, String>,
    metrics: HashMap<String, f64>,
}

impl Record {
    pub fn new(
        timestamp: Timestamp,
        labels: HashMap<String, String>,
        metrics: HashMap<String, f64>,
    ) -> Self {
        Self {
            timestamp,
            labels,
            metrics,
        }
    }

    pub fn label(&self, name: &String) -> Option<&String> {
        self.labels.get(name)
    }
}
