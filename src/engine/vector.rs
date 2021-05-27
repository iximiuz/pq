use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use super::value::Value;
use crate::common::time::TimeRange;
use crate::input::{Cursor, Sample};
use crate::model::types::{Instant, Timestamp};
use crate::parser::ast::VectorSelector;

pub(super) struct VectorSelectorExecutor {
    selector: VectorSelector,
    cursor: Rc<Cursor>,
    interval: Duration,
    lookback: Duration,
    next_instant: Option<Timestamp>,
    last_instant: Option<Timestamp>,
    buffer: VecDeque<Rc<Sample>>,
}

impl VectorSelectorExecutor {
    pub fn new(
        selector: VectorSelector,
        cursor: Rc<Cursor>,
        range: TimeRange,
        interval: Duration,
        lookback: Duration,
    ) -> Self {
        Self {
            selector,
            cursor,
            interval,
            lookback,
            next_instant: range.start(),
            last_instant: range.end(),
            buffer: VecDeque::new(),
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
        // TODO: turn buffer into instant vector
        None
    }
}

impl std::iter::Iterator for VectorSelectorExecutor {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let sample = match self.next_sample() {
                Some(sample) => sample,
                None => return self.finalize(), // drained input
            };

            if let Some(last_instant) = self.last_instant {
                // Input not drained, but we've seen enough.
                if sample.timestamp > last_instant {
                    return self.finalize();
                }
            }

            // Maybe fixup next_instant on the very first iteration.
            let next_instant = match self.next_instant {
                Some(next_instant) => next_instant,
                None => ((sample.timestamp - 1) as f64 / 1000.0) as Timestamp + 1,
            };

            let outdated_instant = next_instant.add(self.lookback);
            if sample.timestamp <= next_instant {
                if sample.timestamp > outdated_instant {
                    self.buffer.push_back(sample);
                }
                continue;
            }

            let rv = self.create_vector_from_buffer();
            self.next_instant = self.next_instant.add(self.interval);
            self.purge_samples_behind_lookback();
            self.buffer.push_back(sample);
            return rv;
        }
    }
}
