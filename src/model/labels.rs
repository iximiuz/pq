use std::collections::{BTreeSet, HashMap, HashSet};

use regex::Regex;

use crate::error::{Error, Result};

const NAME_LABEL: &str = "__name__";

pub type LabelName = String;

pub type LabelValue = String;

pub type Labels = HashMap<LabelName, LabelValue>;

// TODO: use BTree* everywhere.

pub trait LabelsTrait {
    fn with(&self, names: &HashSet<LabelName>) -> Self;
    fn without(&self, names: &HashSet<LabelName>) -> Self;
    fn name(&self) -> Option<&LabelName>;
    fn set_name(&mut self, name: LabelName);
    fn drop_name(&mut self);
    fn to_vec(&self) -> Vec<u8>;
}

impl LabelsTrait for Labels {
    fn with(&self, names: &HashSet<LabelName>) -> Self {
        let mut labels = self.clone();
        labels.retain(|name, _| name != NAME_LABEL && names.contains(name));
        labels
    }

    fn without(&self, names: &HashSet<LabelName>) -> Self {
        let mut labels = self.clone();
        labels.retain(|name, _| name != NAME_LABEL && !names.contains(name));
        labels
    }

    fn name(&self) -> Option<&LabelValue> {
        self.get(NAME_LABEL)
    }

    fn set_name(&mut self, name: LabelName) {
        self.insert(NAME_LABEL.to_string(), name);
    }

    fn drop_name(&mut self) {
        self.remove(NAME_LABEL);
    }

    fn to_vec(&self) -> Vec<u8> {
        let sorted: BTreeSet<_> = self.clone().into_iter().collect();
        sorted
            .into_iter()
            .flat_map(|(name, value)| [name.as_bytes(), &[b'\xFF'], value.as_bytes()].concat())
            .collect()
    }
}

#[derive(Debug)]
pub struct LabelMatcher {
    label: LabelName,
    match_op: MatchOp,
    value: LabelValue,
    re: Option<Regex>,
}

impl LabelMatcher {
    pub fn new<N, V>(label: N, match_op: MatchOp, value: V) -> Result<Self>
    where
        N: Into<LabelName>,
        V: Into<LabelValue>,
    {
        let label = label.into();
        let value = value.into();

        assert!(!label.is_empty());

        let re = match match_op {
            MatchOp::EqlRe | MatchOp::NeqRe => {
                Some(Regex::new(&format!("^(?:{})$", value)).map_err(|e| format!("{}", e))?)
            }
            _ => None,
        };

        Ok(Self {
            label,
            match_op,
            value,
            re,
        })
    }

    pub fn name_matcher<V>(name: V) -> Self
    where
        V: Into<LabelValue>,
    {
        let name = name.into();
        assert!(!name.is_empty());

        Self {
            label: NAME_LABEL.to_string(),
            match_op: MatchOp::Eql,
            value: name,
            re: None,
        }
    }

    pub fn label(&self) -> &LabelName {
        &self.label
    }

    pub fn match_op(&self) -> &MatchOp {
        &self.match_op
    }

    pub fn value(&self) -> &LabelValue {
        &self.value
    }

    pub fn is_name_matcher(&self) -> bool {
        self.label == NAME_LABEL
    }

    pub fn matches(&self, v: &str) -> bool {
        match self.match_op {
            MatchOp::Eql => self.value == v,
            MatchOp::Neq => self.value != v,
            MatchOp::EqlRe => self
                .re
                .as_ref()
                .expect("some regex is always expected for this type of matcher")
                .is_match(v),
            MatchOp::NeqRe => !self
                .re
                .as_ref()
                .expect("some regex is always expected for this type of matcher")
                .is_match(v),
        }
    }
}

impl PartialEq for LabelMatcher {
    fn eq(&self, other: &Self) -> bool {
        self.label == other.label && self.match_op == other.match_op && self.value == other.value
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MatchOp {
    Eql,
    Neq,
    EqlRe,
    NeqRe,
}

impl std::convert::TryFrom<&str> for MatchOp {
    type Error = Error;

    fn try_from(op: &str) -> Result<Self> {
        match op {
            "=" => Ok(MatchOp::Eql),
            "!=" => Ok(MatchOp::Neq),
            "=~" => Ok(MatchOp::EqlRe),
            "!~" => Ok(MatchOp::NeqRe),
            _ => Err(Error::new("Unexpected match op literal")),
        }
    }
}
