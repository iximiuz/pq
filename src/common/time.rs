use crate::error::Result;
use crate::model::types::Timestamp;

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
