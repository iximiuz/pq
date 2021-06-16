use std::collections::HashSet;

use crate::error::{Error, Result};
use crate::model::{
    labels::{LabelMatcher, LabelName},
    types::{MetricName, SampleValue},
};

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
    BinaryExpr(BinaryExpr),
    NumberLiteral(SampleValue),
    UnaryExpr(UnaryOp, Box<Expr>),
    VectorSelector(VectorSelector),

    /// Never appears in the query language. Used in the engine for some
    /// optimization.
    Noop,
}

#[derive(Debug, PartialEq)]
pub struct VectorSelector {
    matchers: Vec<LabelMatcher>,
}

impl VectorSelector {
    pub fn new<S>(name: Option<S>, mut matchers: Vec<LabelMatcher>) -> Result<Self>
    where
        S: Into<MetricName>,
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
pub struct BinaryExpr {
    lhs: Box<Expr>,
    rhs: Box<Expr>,
    op: BinaryOp,
    bool_modifier: bool,
    label_matching: Option<LabelMatching>,
    group_modifier: Option<GroupModifier>,
}

impl BinaryExpr {
    pub fn new(lhs: Expr, op: BinaryOp, rhs: Expr) -> Self {
        Self::new_ex(lhs, op, rhs, false, None, None)
    }

    pub fn new_ex(
        lhs: Expr,
        op: BinaryOp,
        rhs: Expr,
        bool_modifier: bool,
        label_matching: Option<LabelMatching>,
        group_modifier: Option<GroupModifier>,
    ) -> Self {
        assert!(!bool_modifier || op.kind() == BinaryOpKind::Comparison);
        assert!(group_modifier.is_none() || label_matching.is_some());
        assert!(group_modifier.is_none() || op.kind() != BinaryOpKind::Logical);

        Self {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            op,
            bool_modifier,
            label_matching,
            group_modifier,
        }
    }

    #[inline]
    pub fn op(&self) -> BinaryOp {
        self.op
    }

    #[inline]
    pub fn lhs(&self) -> &Expr {
        self.lhs.as_ref()
    }

    #[inline]
    pub fn rhs(&self) -> &Expr {
        self.rhs.as_ref()
    }

    #[inline]
    pub fn into_inner(
        self,
    ) -> (
        BinaryOp,
        Box<Expr>,
        Box<Expr>,
        bool,
        Option<LabelMatching>,
        Option<GroupModifier>,
    ) {
        (
            self.op,
            self.lhs,
            self.rhs,
            self.bool_modifier,
            self.label_matching,
            self.group_modifier,
        )
    }
}

#[derive(Debug, PartialEq)]
pub enum LabelMatching {
    On(HashSet<LabelName>),
    Ignoring(HashSet<LabelName>),
}

#[derive(Debug, PartialEq)]
pub enum GroupModifier {
    Left(Vec<LabelName>),
    Right(Vec<LabelName>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnaryOp {
    Add,
    Sub,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BinaryOp {
    Add,
    Div,
    Mul,
    Mod,
    Pow,
    Sub,
    Eql,
    Gte,
    Gtr,
    Lss,
    Lte,
    Neq,
    And,
    Unless,
    Or,
}

#[derive(Debug, PartialEq)]
pub enum BinaryOpKind {
    Arithmetic,
    Comparison,
    Logical,
}

pub(super) type Precedence = usize;

impl BinaryOp {
    #[inline]
    pub(super) fn precedence(self) -> Precedence {
        use BinaryOp::*;

        match self {
            Or => 10,
            And | Unless => 20,
            Eql | Gte | Gtr | Lss | Lte | Neq => 30,
            Add | Sub => 40,
            Mul | Div | Mod => 50,
            Pow => 60,
        }
    }

    #[inline]
    pub fn kind(self) -> BinaryOpKind {
        use BinaryOp::*;
        use BinaryOpKind::*;

        match self {
            Add | Sub | Mul | Div | Mod | Pow => Arithmetic,
            Eql | Gte | Gtr | Lss | Lte | Neq => Comparison,
            And | Unless | Or => Logical,
        }
    }
}

/// Try to parse a string into a binary op.
///
/// ```
/// # use std::convert::TryFrom;
/// # use pq::parser::ast::BinaryOp;
/// #
/// # fn main() {
/// let op = BinaryOp::try_from("+");
/// assert_eq!(BinaryOp::Add, op.unwrap());
///
/// let op = BinaryOp::try_from("==");
/// assert_eq!(BinaryOp::Eql, op.unwrap());
///
/// let op = BinaryOp::try_from("uNLeSs");
/// assert_eq!(BinaryOp::Unless, op.unwrap());
/// # }
impl std::convert::TryFrom<&str> for BinaryOp {
    type Error = Error;

    fn try_from(op: &str) -> Result<Self> {
        use BinaryOp::*;

        match op.to_lowercase().as_str() {
            "+" => Ok(Add),
            "/" => Ok(Div),
            "*" => Ok(Mul),
            "%" => Ok(Mod),
            "^" => Ok(Pow),
            "-" => Ok(Sub),
            "==" => Ok(Eql),
            ">=" => Ok(Gte),
            ">" => Ok(Gtr),
            "<" => Ok(Lss),
            "<=" => Ok(Lte),
            "!=" => Ok(Neq),
            "and" => Ok(And),
            "unless" => Ok(Unless),
            "or" => Ok(Or),
            _ => Err(Error::new("Unexpected binary op literal")),
        }
    }
}
