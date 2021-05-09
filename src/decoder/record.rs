use std::collections::HashMap;

type Timestamp = i64;

#[derive(Debug)]
pub struct Record {
    timestamp: Timestamp,
    labels: HashMap<String, String>,
    metrics: Vec<Metric>,
}

impl Record {
    pub fn new(
        timestamp: Timestamp,
        labels: HashMap<String, String>,
        metrics: HashMap<String, f64>,
    ) -> Self {
        let metrics = metrics
            .into_iter()
            .map(|m| Metric {
                name: m.0,
                value: m.1,
                labels: labels.clone(),
            })
            .collect();
        Self {
            timestamp,
            labels,
            metrics,
        }
    }

    pub fn metrics(&self) -> &Vec<Metric> {
        &self.metrics
    }
}

#[derive(Debug)]
pub struct Metric {
    name: String,
    value: f64,
    labels: HashMap<String, String>,
}

impl Metric {
    pub fn label(&self, name: &str) -> Option<&String> {
        match name {
            "__name__" => Some(&self.name),
            _ => self.labels.get(name),
        }
    }
}
