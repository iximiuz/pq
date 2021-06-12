use regex::Regex;

use super::types::{LabelName, LabelValue};
use crate::error::{Error, Result};

const LABEL_NAME: &str = "__name__";

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

        assert!(label.len() > 0);

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
        assert!(name.len() > 0);

        Self {
            label: LABEL_NAME.to_string(),
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
        self.label == LABEL_NAME
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
