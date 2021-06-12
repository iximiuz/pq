use crate::model::types::{LabelName, Labels, SampleValue, Timestamp};

// Every Expr can be evaluated to a value.
#[derive(Debug)]
pub enum ExprValue {
    InstantVector(InstantVector),
    RangeVector(RangeVector),
    Scalar(SampleValue),
    // String(String)
}

#[derive(Debug, PartialEq)]
pub(super) enum ExprValueKind {
    InstantVector,
    RangeVector,
    Scalar,
}

pub(super) trait ExprValueIter: std::iter::Iterator<Item = ExprValue> {
    fn value_kind(&self) -> ExprValueKind;
}

#[derive(Debug)]
pub struct InstantVector {
    instant: Timestamp,
    samples: Vec<(Labels, SampleValue)>,
}

impl InstantVector {
    pub fn new(instant: Timestamp, samples: Vec<(Labels, SampleValue)>) -> Self {
        Self { instant, samples }
    }

    #[inline]
    pub fn timestamp(&self) -> Timestamp {
        self.instant
    }

    #[inline]
    pub fn samples(&self) -> &[(Labels, SampleValue)] {
        return &self.samples;
    }

    pub fn mutate_values(&mut self, f: impl FnMut(&mut (Labels, SampleValue))) {
        self.samples.iter_mut().for_each(f)
    }

    pub fn match_vector(
        &self,
        other: &InstantVector,
        _on: Vec<LabelName>,
        _ignoring: Vec<LabelName>,
        f: impl Fn(SampleValue, SampleValue) -> SampleValue,
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
