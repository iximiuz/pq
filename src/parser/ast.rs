use crate::error::{Error, Result};
use crate::model::{
    labels::LabelMatcher,
    types::{LabelName, MetricName, SampleValue},
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
    vector_matching: Option<VectorMatching>,
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
        vector_matching: Option<VectorMatching>,
        group_modifier: Option<GroupModifier>,
    ) -> Self {
        assert!(!bool_modifier || op.is_comparison());
        assert!(group_modifier.is_none() || vector_matching.is_some());
        assert!(group_modifier.is_none() || !op.is_logical());

        Self {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
            op,
            bool_modifier,
            vector_matching,
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
        Box<Expr>,
        BinaryOp,
        Box<Expr>,
        bool,
        Option<VectorMatching>,
        Option<GroupModifier>,
    ) {
        (
            self.lhs,
            self.op,
            self.rhs,
            self.bool_modifier,
            self.vector_matching,
            self.group_modifier,
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum VectorMatchingKind {
    On,
    Ignoring,
}

/// Try to parse a string into a VectorMatchingKind.
///
/// ```
/// # use std::convert::TryFrom;
/// # use pq::parser::ast::VectorMatchingKind;
/// #
/// # fn main() {
/// let kind = VectorMatchingKind::try_from("on");
/// assert_eq!(VectorMatchingKind::On, kind.unwrap());
///
/// let kind = VectorMatchingKind::try_from("iGnOrInG");
/// assert_eq!(VectorMatchingKind::Ignoring, kind.unwrap());
/// # }
impl std::convert::TryFrom<&str> for VectorMatchingKind {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self> {
        use VectorMatchingKind::*;

        match s.to_lowercase().as_str() {
            "on" => Ok(On),
            "ignoring" => Ok(Ignoring),
            _ => Err(Error::new("Unexpected vector matching kind")),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct VectorMatching {
    kind: VectorMatchingKind,
    labels: Vec<LabelName>,
}

impl VectorMatching {
    pub fn new(kind: VectorMatchingKind, labels: Vec<LabelName>) -> Self {
        Self { kind, labels }
    }
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
    pub fn is_arithmetic(self) -> bool {
        !(self.is_comparison() || self.is_logical())
    }

    #[inline]
    pub fn is_comparison(self) -> bool {
        use BinaryOp::*;

        match self {
            Eql | Gte | Gtr | Lss | Lte | Neq => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_logical(self) -> bool {
        use BinaryOp::*;

        match self {
            And | Unless | Or => true,
            _ => false,
        }
    }
}

// impl fmt::Display for BinaryOp {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         use BinaryOp::*;
//
//         match self {
//             Add => write!(f, "+"),
//             Div => write!(f, "/"),
//             Mul => write!(f, "*"),
//             Mod => write!(f, "%"),
//             Pow => write!(f, "^"),
//             Sub => write!(f, "-"),
//             Eql => write!(f, "=="),
//             Gte => write!(f, ">="),
//             Gtr => write!(f, ">"),
//             Lss => write!(f, "<"),
//             Lte => write!(f, "<="),
//             Neq => write!(f, "!="),
//             And => write!(f, "and"),
//             Unless => write!(f, "unless"),
//             Or => write!(f, "or"),
//         }
//     }
// }

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
