use std::time::Duration;

use chrono::prelude::*;

// Unix timestamp in milliseconds.
pub type Timestamp = i64;

pub trait TimestampTrait {
    fn add(&self, d: Duration) -> Self;
    fn sub(&self, d: Duration) -> Self;
    fn round_up_to_secs(&self) -> Self;
    fn to_string_millis(&self) -> String;
}

impl TimestampTrait for Timestamp {
    #[inline]
    fn add(&self, d: Duration) -> Self {
        // TODO: check for i64 overflow
        *self + 1000 * d.as_secs() as i64 + d.subsec_millis() as i64
    }

    #[inline]
    fn sub(&self, d: Duration) -> Self {
        // TODO: check for i64 overflow
        *self - 1000 * d.as_secs() as i64 - d.subsec_millis() as i64
    }

    #[inline]
    fn round_up_to_secs(&self) -> Self {
        (((*self - 1) as f64 / 1000.0) as i64 + 1) * 1000
    }

    fn to_string_millis(&self) -> String {
        let ts = NaiveDateTime::from_timestamp(self / 1000, 0);
        ts.format("%Y-%m-%dT%H:%M:%S%.3f").to_string()
    }
}
