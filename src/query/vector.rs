use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use std::time::Duration;

use super::parser::ast::VectorSelector;
use super::sample::{Cursor, Sample};
use super::value::{InstantVector, QueryValue, QueryValueIter, QueryValueKind, RangeVector};
use crate::model::{Labels, LabelsTrait, SampleValue, Timestamp, TimestampTrait};

pub(super) struct VectorSelectorEvaluator {
    cursor: Rc<Cursor>,
    selector: VectorSelector,
    interval: Duration,
    lookback: Duration,
    next_instant: Option<Timestamp>,
    buffer: SampleMatrix,
}

impl VectorSelectorEvaluator {
    pub(super) fn new(
        cursor: Rc<Cursor>,
        selector: VectorSelector,
        interval: Duration,
        lookback: Duration,
        start_at: Option<Timestamp>,
    ) -> Self {
        // Configurable lookback is only allowed for instant vectors.
        // If the provided selector is a range selector, override the
        // lookback duration.
        let lookback = selector.duration().unwrap_or(lookback);

        // Trying to set a reasonable first instant timestamp if the
        // query time range is provided. We cannot start from start_at
        // as is because the very first vector would have to few samples.
        let next_instant =
            start_at.and_then(|t| Some(t.add(std::cmp::min(lookback, interval)) - 1));

        Self {
            cursor,
            selector,
            interval,
            lookback,
            next_instant,
            buffer: SampleMatrix::new(),
        }
    }

    fn next_sample(&self) -> Option<Rc<Sample>> {
        loop {
            let sample = match self.cursor.read() {
                Some(s) => s,
                None => return None, // drained input
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
}

impl std::iter::Iterator for VectorSelectorEvaluator {
    type Item = QueryValue;

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
                // Trying to set a reasonable first instant timestamp if the
                // query time range is provided. We cannot start from start_at
                // as is because the very first vector would have to few samples.
                self.next_instant = Some(
                    sample
                        .timestamp()
                        .add(std::cmp::min(self.lookback, self.interval))
                        - 1,
                );
            }

            // This sample timestamp check is more an optimization than a necessity.
            if sample.timestamp() > self.next_instant.unwrap().sub(self.lookback) {
                self.buffer.push(sample);
            }
        }

        if self.buffer.is_empty() {
            return None;
        }

        // Here we have a sample after the current next_instant.
        // Hence, we can create (a potentially empty) vector from the current buffer.
        let vector = match self.selector.duration() {
            None => QueryValue::InstantVector(
                self.buffer
                    .instant_vector(self.next_instant.unwrap(), self.lookback),
            ),
            Some(duration) => QueryValue::RangeVector(
                self.buffer
                    .range_vector(self.next_instant.unwrap(), duration),
            ),
        };

        // Advance next_instant for the next iteration.
        self.next_instant = Some(self.next_instant.unwrap().add(self.interval));

        let keep_since = self.next_instant.unwrap().sub(self.lookback);
        self.buffer.purge_before(keep_since);

        return Some(vector);
    }
}

impl QueryValueIter for VectorSelectorEvaluator {
    fn value_kind(&self) -> QueryValueKind {
        match self.selector.duration() {
            None => QueryValueKind::InstantVector,
            Some(_) => QueryValueKind::RangeVector,
        }
    }
}

#[derive(Debug)]
struct SampleMatrix {
    matrix: BTreeMap<Vec<u8>, (Labels, VecDeque<(SampleValue, Timestamp)>)>,
    latest_sample_timestamp: Option<Timestamp>,
}

// TODO: optimize - algorithm!
// TODO: optimize - stop cloning labels!
impl SampleMatrix {
    fn new() -> Self {
        Self {
            matrix: BTreeMap::new(),
            latest_sample_timestamp: None,
        }
    }

    fn is_empty(&self) -> bool {
        assert!((self.matrix.len() == 0) == self.latest_sample_timestamp.is_none());
        self.matrix.len() == 0
    }

    fn latest_sample_timestamp(&self) -> Option<Timestamp> {
        assert!((self.matrix.len() == 0) == self.latest_sample_timestamp.is_none());
        self.latest_sample_timestamp
    }

    fn push(&mut self, sample: Rc<Sample>) {
        self.matrix
            .entry(sample.labels().to_vec())
            .or_insert((sample.labels().clone(), VecDeque::new()))
            .1
            .push_back((sample.value(), sample.timestamp()));

        self.latest_sample_timestamp = Some(sample.timestamp());
    }

    /// Purges samples up until and including `instant`.
    fn purge_before(&mut self, instant: Timestamp) {
        self.matrix.retain(|_, (_, series)| {
            // Tiny optimization - maybe we can clean up the whole key in one go.
            if let Some((_, ts)) = series.back() {
                if *ts <= instant {
                    series.clear();
                }
            }

            loop {
                match series.front() {
                    Some((_, ts)) if *ts <= instant => {
                        series.pop_front();
                    }
                    _ => break,
                }
            }

            !series.is_empty()
        });

        if self.matrix.len() == 0 {
            self.latest_sample_timestamp = None;
        }
    }

    /// Returns samples in the (instant - lookback, instant] time range.
    fn instant_vector(&self, instant: Timestamp, lookback: Duration) -> InstantVector {
        let stale_instant = instant.sub(lookback);

        let samples = self
            .matrix
            .values()
            .filter_map(|(labels, series)| {
                series.iter().rev().find_map(|(val, ts)| {
                    if stale_instant < *ts && *ts <= instant {
                        Some((labels.clone(), *val))
                    } else {
                        None
                    }
                })
            })
            .collect();

        InstantVector::new(instant, samples)
    }

    /// Returns samples in the (instant - range, instant] time range.
    fn range_vector(&self, instant: Timestamp, duration: Duration) -> RangeVector {
        let from_instant = instant.sub(duration);

        let samples = self
            .matrix
            .values()
            .filter_map(|(labels, series)| {
                let range_samples: Vec<(SampleValue, Timestamp)> = series
                    .iter()
                    .cloned()
                    .rev()
                    .filter(|(_, ts)| from_instant < *ts && *ts <= instant)
                    .collect();
                match range_samples.len() {
                    0 => None,
                    _ => Some((labels.clone(), range_samples)),
                }
            })
            .collect();

        RangeVector::new(instant, samples)
    }
}
