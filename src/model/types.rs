use std::time::Duration;

pub type Timestamp = i64;

pub trait Instant {
    fn add(&self, d: Duration) -> Self;
}

impl Instant for Timestamp {
    fn add(&self, d: Duration) -> Self {
        // assert!(d.as_secs() + d.subsec_millis() < Timestamp::MAX);
        *self
    }
}
