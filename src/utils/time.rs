use chrono::prelude::*;

use crate::error::{Error, Result};
use crate::model::Timestamp;

#[derive(Debug, Clone, Copy)]
pub struct TimeRange {
    start: Option<Timestamp>,
    end: Option<Timestamp>,
}

impl TimeRange {
    pub fn new(start: Option<Timestamp>, end: Option<Timestamp>) -> Result<Self> {
        if start.unwrap_or(Timestamp::MIN) > end.unwrap_or(Timestamp::MAX) {
            return Err("end time is before start time".into());
        }
        Ok(Self { start, end })
    }

    pub fn infinity() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    #[inline]
    pub fn start(&self) -> Option<Timestamp> {
        self.start
    }

    #[inline]
    pub fn end(&self) -> Option<Timestamp> {
        self.end
    }
}

pub fn parse_time(s: &str, format: &str) -> Result<Timestamp> {
    if format.contains("%z") {
        Ok(DateTime::parse_from_str(s, format)
            .map_err(|e| (Error::from(("couldn't parse timestamp", e))))?
            .timestamp_millis())
    } else {
        Ok(NaiveDateTime::parse_from_str(s, format)
            .map_err(|e| (Error::from(("couldn't parse timestamp", e))))?
            .timestamp_millis())
    }
}

pub fn try_parse_time(s: &str) -> Option<Timestamp> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis());
    }

    if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
        return Some(dt.timestamp_millis());
    }

    // Nginx
    if let Ok(dt) = DateTime::parse_from_str(s, "%d/%b/%Y:%H:%M:%S %z") {
        return Some(dt.timestamp_millis());
    }

    // ISO-like
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(dt.timestamp_millis());
    }

    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(dt.timestamp_millis());
    }

    if let Ok(dt) = DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f %z") {
        return Some(dt.timestamp_millis());
    }

    // UNIX timestamp
    if s.chars().all(|c| char::is_digit(c, 10)) {
        let n = s.parse::<i64>().unwrap();
        match s.len() {
            10 => return Some(n * 1000),
            13 => return Some(n),
            _ => return None,
        };
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_parse_time() -> std::result::Result<(), Box<dyn std::error::Error>> {
        #[rustfmt::skip]
        let tests = [
            ("2021-01-01 00:00:00", 1609459200000),
            ("2021-01-01 00:00:00.00", 1609459200000),
            ("2021-01-01 00:00:00.00000", 1609459200000),
            ("2021-01-01T00:00:00.00000", 1609459200000),
            ("2021-01-01 01:00:00.00000 +0100", 1609459200000),
            ("2020-12-31 14:30:00.00000 -0930", 1609459200000),
            ("2021-01-01T00:00:00+00:00", 1609459200000),
            ("Fri, 1 Jan 2021 00:00:00 +0000", 1609459200000),
            ("01/Jan/2021:00:00:00 -0000", 1609459200000),
            ("1609459200",    1609459200000),
            ("1609459200100", 1609459200100),
        ];

        for (input, expected) in &tests {
            let actual =
                try_parse_time(input).unwrap_or_else(|| panic!("failed to parse {}", input));
            assert_eq!(*expected, actual);
        }

        Ok(())
    }
}
