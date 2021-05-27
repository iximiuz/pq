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
    finalized: bool,
}

impl VectorSelectorExecutor {
    pub fn new(
        cursor: Rc<Cursor>,
        selector: VectorSelector,
        range: TimeRange,
        interval: Duration,
        lookback: Duration,
    ) -> Self {
        Self {
            cursor,
            selector,
            interval,
            lookback,
            next_instant: range.start().map(|t| t.round_up_to_secs()),
            last_instant: range.end().map(|t| t.round_up_to_secs()),
            buffer: SampleMatrix::new(),
            finalized: false,
        }
    }

    fn next_sample(&self) -> Option<Rc<Sample>> {
        loop {
            let sample = match self.cursor.read() {
                Some(s) => s,
                None => return None,
            };

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

    fn finalize(&mut self) -> Option<Value> {
        if self.finalized {
            return None;
        }

        match self.next_instant {
            Some(next_instant) => {
                self.finalized = true;

                Some(Value::InstantVector(
                    self.buffer.instant_vector(next_instant, self.lookback),
                ))
            }
            None => {
                assert!(self.buffer.is_empty());
                None
            }
        }
    }
}

impl std::iter::Iterator for VectorSelectorExecutor {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        println!("VectorSelectorExecutor::next");
        loop {
            println!("VectorSelectorExecutor::next loop");
            let sample = match self.next_sample() {
                Some(sample) => sample,
                None => return self.finalize(), // drained input
            };
            println!("VectorSelectorExecutor::next SAMPLE={:?}", sample);
            let sample_timestamp = sample.timestamp();

            // Input not drained, but we've seen enough.
            if sample_timestamp > self.last_instant.unwrap_or(Timestamp::MAX) {
                return self.finalize();
            }

            let next_instant = self
                .next_instant
                // Maybe fixup next_instant. Can happen
                // only on the very first next() call.
                .unwrap_or(sample_timestamp.round_up_to_secs());

            assert!(next_instant <= self.last_instant.unwrap_or(Timestamp::MAX));

            // This check is more like an optimization than a necessity.
            if sample_timestamp > next_instant.sub(self.lookback) {
                self.buffer.push(sample);
            }

            if sample_timestamp > next_instant {
                // Here we have a sample after the current next_instant.
                // Hence, we can create (a potentially empty) vector from the current buffer.
                let samples = self.buffer.instant_vector(next_instant, self.lookback);

                // Advance next_instant for the next iteration.
                self.next_instant = Some(next_instant.add(self.interval));

                self.buffer
                    .purge_stale(self.next_instant.unwrap(), self.lookback);

                return Some(Value::InstantVector(samples));
            }
        }
    }
}

struct SampleMatrix {
    samples: HashMap<String, (Labels, VecDeque<(Timestamp, f64)>)>,
}

// TODO: optimize me!
impl SampleMatrix {
    fn new() -> Self {
        Self {
            samples: HashMap::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.samples.len() == 0
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
    }

    /// Returns samples in the (instant - lookback, instant] time range.
    fn instant_vector(&self, instant: Timestamp, lookback: Duration) -> InstantVector {
        InstantVector::new(instant)
    }

    /// Purges samples up until and including `next_instant - lookback` duration.
    fn purge_stale(&mut self, next_instant: Timestamp, lookback: Duration) {}
}
