use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::time::Duration;

use super::value::{InstantVector, Value};
use crate::common::time::TimeRange;
use crate::input::{Cursor, Sample};
use crate::model::types::{Instant, Labels, Timestamp};
use crate::parser::ast::VectorSelector;

pub(super) struct VectorSelectorExecutor {
    cursor: Rc<Cursor>,
    selector: VectorSelector,
    interval: Duration,
    lookback: Duration,
    next_instant: Option<Timestamp>,
    last_instant: Option<Timestamp>,
    buffer: SampleMatrix,
}

impl VectorSelectorExecutor {
    pub fn new(
        cursor: Rc<Cursor>,
        selector: VectorSelector,
        range: TimeRange,
        interval: Duration,
        lookback: Duration,
    ) -> Self {
        assert!(
            lookback.as_secs() > 0,
            "lookbacks < 1 sec aren't supported yet"
        );

        Self {
            cursor,
            selector,
            interval,
            lookback,
            next_instant: range.start().map(|t| t.round_up_to_secs()),
            last_instant: range.end().map(|t| t.round_up_to_secs()),
            buffer: SampleMatrix::new(),
        }
    }

    fn next_sample(&self) -> Option<Rc<Sample>> {
        loop {
            let sample = match self.cursor.read() {
                Some(s) => s,
                None => return None, // drained input
            };

            if sample.timestamp() > self.last_instant.unwrap_or(Timestamp::MAX) {
                // Input not really drained, but we've seen enough.
                return None;
            }

            if self
                .selector
                .matchers()
                .iter()
                .all(|m| match sample.label(m.label()) {
                    Some(v) => m.matches(v),
                    None => false,
                })
            {
                return Some(sample);
            }
        }
    }
}

impl std::iter::Iterator for VectorSelectorExecutor {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        while self
            .buffer
            .latest_sample_timestamp()
            .unwrap_or(Timestamp::MIN)
            <= self.next_instant.unwrap_or(Timestamp::MIN)
        {
            let sample = match self.next_sample() {
                Some(sample) => sample,
                None => break,
            };

            if self.next_instant.is_none() {
                // Maybe fixup next_instant. Can happen only on the very first next() call.
                // FIXME: round_up_to_secs doesn't play well with sub-secondly lookbacks.
                //        To supported sub-secondly lookbacks, the round up should be till
                //        the next even lookback.
                self.next_instant = Some(sample.timestamp().round_up_to_secs());
                assert!(self.next_instant.unwrap() <= self.last_instant.unwrap_or(Timestamp::MAX));
            }

            // The sample's timestamp check is more an optimization than a necessity.
            if sample.timestamp() > self.next_instant.unwrap().sub(self.lookback) {
                self.buffer.push(sample);
            }
        }

        if self.buffer.is_empty() {
            return None;
        }

        // Here we have a sample after the current next_instant.
        // Hence, we can create (a potentially empty) vector from the current buffer.
        let vector = self
            .buffer
            .instant_vector(self.next_instant.unwrap(), self.lookback);

        // Advance next_instant for the next iteration.
        self.next_instant = Some(self.next_instant.unwrap().add(self.interval));

        self.buffer
            .purge_stale(self.next_instant.unwrap(), self.lookback);

        return Some(Value::InstantVector(vector));
    }
}

struct SampleMatrix {
    samples: HashMap<String, (Labels, VecDeque<(Timestamp, f64)>)>,
    latest_sample_timestamp: Option<Timestamp>,
}

// TODO: optimize me!
impl SampleMatrix {
    fn new() -> Self {
        Self {
            samples: HashMap::new(),
            latest_sample_timestamp: None,
        }
    }

    fn is_empty(&self) -> bool {
        assert!((self.samples.len() == 0) == self.latest_sample_timestamp.is_none());
        self.samples.len() == 0
    }

    fn latest_sample_timestamp(&self) -> Option<Timestamp> {
        assert!((self.samples.len() == 0) == self.latest_sample_timestamp.is_none());
        self.latest_sample_timestamp
    }

    fn push(&mut self, sample: Rc<Sample>) {
        let key = sample
            .labels()
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join(";");

        self.samples
            .entry(key)
            .or_insert((sample.labels().clone(), VecDeque::new()))
            .1
            .push_back((sample.timestamp(), sample.value()));

        self.latest_sample_timestamp = Some(sample.timestamp());
    }

    /// Returns samples in the (instant - lookback, instant] time range.
    fn instant_vector(&self, instant: Timestamp, lookback: Duration) -> InstantVector {
        InstantVector::new(instant)
    }

    /// Purges samples up until and including `next_instant - lookback` duration.
    fn purge_stale(&mut self, next_instant: Timestamp, lookback: Duration) {}
}