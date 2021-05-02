use crate::error::{Error, Result};

#[derive(Debug)]
pub struct AST {
    pub root: NodeKind,
}

impl AST {
    pub fn new(root: NodeKind) -> Self {
        Self { root }
    }
}

#[derive(Debug)]
pub enum NodeKind {
    VectorSelector(VectorSelector),
}

#[derive(Debug)]
pub struct VectorSelector {
    metric: Option<String>,
    labels: LabelMatchers,
}

impl VectorSelector {
    pub fn new(metric: Option<String>, labels: LabelMatchers) -> Result<Self> {
        if metric.is_none() && labels.is_match_all() {
            return Err(Error::new(
                "vector selector must contain at least one non-empty matcher",
            ));
        }

        Ok(Self { metric, labels })
    }

    pub fn labels(&self) -> &LabelMatchers {
        &self.labels
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct LabelMatchers {
    matchers: Vec<LabelMatcher>,
}

type MatcherIterator<'a> = std::slice::Iter<'a, LabelMatcher>;

impl<'a> LabelMatchers {
    pub fn new(matchers: Vec<LabelMatcher>) -> Self {
        Self { matchers }
    }

    pub fn is_match_all(&self) -> bool {
        self.matchers.len() == 0
    }

    pub fn iter(&self) -> MatcherIterator {
        self.matchers.iter()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct LabelMatcher {
    label: String,
    match_op: MatchOp,
    value: String,
}

impl LabelMatcher {
    pub fn new<S>(label_str: S, match_op: MatchOp, value_str: S) -> Self
    where
        S: Into<String>,
    {
        let label = label_str.into();
        let value = value_str.into();

        // TODO: check it's not a match-all matcher

        assert!(label.len() > 0);
        assert!(value.len() > 0);

        Self {
            label,
            match_op,
            value,
        }
    }

    pub fn label(&self) -> &String {
        &self.label
    }

    pub fn match_op(&self) -> &MatchOp {
        &self.match_op
    }

    pub fn value(&self) -> &String {
        &self.value
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MatchOp {
    Eql,
    Neq,
    EqlRe,
    NeqRe,
}
