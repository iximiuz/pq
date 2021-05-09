use crate::error::{Error, Result};
use crate::labels::LabelMatcher;

#[derive(Debug)]
pub struct AST {
    pub root: Expr,
}

impl AST {
    pub fn new(root: Expr) -> Self {
        Self { root }
    }
}

#[derive(Debug, PartialEq)]
pub enum Expr {
    UnaryExpr(UnaryOp, Box<Expr>),
    VectorSelector(VectorSelector),
}

#[derive(Debug, PartialEq)]
pub struct VectorSelector {
    matchers: Vec<LabelMatcher>,
}

impl VectorSelector {
    pub fn new<S>(name: Option<S>, mut matchers: Vec<LabelMatcher>) -> Result<Self>
    where
        S: Into<String>,
    {
        let (matches_everything, has_name_matcher) =
            matchers.iter().fold((true, false), |(me, hnm), m| {
                (me && m.matches(""), hnm || m.is_name_matcher())
            });

        if name.is_some() && has_name_matcher {
            return Err(Error::new("potentially ambiguous metric name match"));
        }

        if name.is_none() && matches_everything {
            return Err(Error::new(
                "vector selector must contain at least one non-empty matcher",
            ));
        }

        if let Some(name) = name {
            matchers.push(LabelMatcher::name_matcher(name));
        }

        Ok(Self { matchers })
    }

    pub fn matchers(&self) -> &Vec<LabelMatcher> {
        &self.matchers
    }
}

#[derive(Debug, PartialEq)]
pub enum UnaryOp {
    Add,
    Sub,
}
