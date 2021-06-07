use crate::model::types::{Labels, Timestamp, Value};

// Every Expr can be evaluated to a Value.
#[derive(Debug)]
pub enum ValueKind {
    InstantVector(InstantVector),
    RangeVector(RangeVector),
    Scalar(Value),
    // String(String),
}

pub(super) type ValueIter = Box<dyn std::iter::Iterator<Item = ValueKind>>;

#[derive(Debug)]
pub struct InstantVector {
    instant: Timestamp,
    samples: Vec<(Labels, Value)>,
}

impl InstantVector {
    pub fn new(instant: Timestamp, samples: Vec<(Labels, Value)>) -> Self {
        Self { instant, samples }
    }

    #[inline]
    pub fn timestamp(&self) -> Timestamp {
        self.instant
    }

    #[inline]
    pub fn samples(&self) -> &[(Labels, Value)] {
        return &self.samples;
    }

    pub fn mutate_values(&mut self, f: impl FnMut(&mut (Labels, Value))) {
        self.samples.iter_mut().for_each(f)
    }

    pub fn match_vector(
        &self,
        other: &InstantVector,
        _on: Vec<String>,
        _ignoring: Vec<String>,
        f: impl Fn(Value, Value) -> Value,
    ) -> Self {
        // println!("match vector {:?} {:?}", self, other);
        assert!(self.instant == other.instant);

        // TODO: implement proper label matching!
        let samples = self
            .samples
            .iter()
            .zip(other.samples.iter())
            .map(|((ll, lv), (_, rv))| (ll.clone(), f(*lv, *rv)))
            .collect();

        InstantVector::new(self.instant, samples)
    }
}

#[derive(Debug)]
pub struct RangeVector {}
