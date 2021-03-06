use std::collections::HashSet;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::model::{LabelMatcher, LabelName, MetricName, SampleValue};

#[derive(Debug, PartialEq)]
pub enum Expr {
    AggregateOperation(AggregateOperation),
    BinaryOperation(BinaryOperation),
    FunctionCall(FunctionCall),
    NumberLiteral(SampleValue),
    Parentheses(Box<Expr>),
    UnaryOperation(UnaryOp, Box<Expr>),
    VectorSelector(VectorSelector),
}

#[derive(Debug, PartialEq)]
pub struct AggregateOperation {
    op: AggregateOp,
    expr: Box<Expr>,
    modifier: Option<AggregateModifier>,
    argument: Option<AggregateArgument>,
}

impl AggregateOperation {
    pub(super) fn new(
        op: AggregateOp,
        expr: Expr,
        modifier: Option<AggregateModifier>,
        argument: Option<AggregateArgument>,
    ) -> Self {
        assert!(op != AggregateOp::CountValues || argument.is_some()); // TODO: arg is string
        assert!(op != AggregateOp::TopK || argument.is_some()); // TODO: arg is number
        assert!(op != AggregateOp::BottomK || argument.is_some()); // TODO: arg is number
        assert!(op != AggregateOp::Quantile || argument.is_some()); // TODO: arg is number
        Self {
            op,
            expr: Box::new(expr),
            modifier,
            argument,
        }
    }

    #[inline]
    pub fn expr(&self) -> &Expr {
        self.expr.as_ref()
    }

    pub fn into_inner(
        self,
    ) -> (
        AggregateOp,
        Box<Expr>,
        Option<AggregateModifier>,
        Option<AggregateArgument>,
    ) {
        (self.op, self.expr, self.modifier, self.argument)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AggregateOp {
    Avg,
    BottomK,
    Count,
    CountValues,
    Group,
    Max,
    Min,
    Quantile,
    StdDev,
    StdVar,
    Sum,
    TopK,
}

impl std::convert::TryFrom<&str> for AggregateOp {
    type Error = Error;

    fn try_from(op: &str) -> Result<Self> {
        use AggregateOp::*;

        match op.to_lowercase().as_str() {
            "avg" => Ok(Avg),
            "bottomk" => Ok(BottomK),
            "count" => Ok(Count),
            "count_values" => Ok(CountValues),
            "group" => Ok(Group),
            "max" => Ok(Max),
            "min" => Ok(Min),
            "quantile" => Ok(Quantile),
            "stddev" => Ok(StdDev),
            "stdvar" => Ok(StdVar),
            "sum" => Ok(Sum),
            "topk" => Ok(TopK),
            _ => Err(Error::new("Unknown aggregate op")),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AggregateModifier {
    By(HashSet<LabelName>),
    Without(HashSet<LabelName>),
}

#[derive(Debug, PartialEq)]
pub enum AggregateArgument {
    String(LabelName),
    Number(f64),
}

#[derive(Debug, PartialEq)]
pub struct BinaryOperation {
    lhs: Box<Expr>,
    rhs: Box<Expr>,
    op: BinaryOp,
    bool_modifier: bool,
    label_matching: Option<LabelMatching>,
    group_modifier: Option<GroupModifier>,
}

impl BinaryOperation {
    #[allow(dead_code)] // It's used in tests.
    pub(super) fn new(lhs: Expr, op: BinaryOp, rhs: Expr) -> Self {
        Self::new_ex(lhs, op, rhs, false, None, None)
    }

    pub(super) fn new_ex(
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
    #[allow(dead_code)] // It's used in tests.
    pub fn op(&self) -> BinaryOp {
        self.op
    }

    #[inline]
    #[allow(dead_code)] // It's used in tests.
    pub fn lhs(&self) -> &Expr {
        self.lhs.as_ref()
    }

    #[inline]
    #[allow(dead_code)] // It's used in tests.
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
            _ => Err(Error::new("Unknown binary op")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FunctionName {
    Clamp,
    ClampMax,
    ClampMin,
    LabelReplace,
    Vector,

    // agg over time
    AvgOverTime,
    CountOverTime,
    LastOverTime,
    MaxOverTime,
    MinOverTime,
    SumOverTime,
}

impl std::convert::TryFrom<&str> for FunctionName {
    type Error = Error;

    fn try_from(name: &str) -> Result<Self> {
        use FunctionName::*;

        match name.to_lowercase().as_str() {
            "clamp" => Ok(Clamp),
            "clamp_max" => Ok(ClampMax),
            "clamp_min" => Ok(ClampMin),
            "label_replace" => Ok(LabelReplace),
            "vector" => Ok(Vector),
            // agg over time
            "avg_over_time" => Ok(AvgOverTime),
            "count_over_time" => Ok(CountOverTime),
            "last_over_time" => Ok(LastOverTime),
            "max_over_time" => Ok(MaxOverTime),
            "min_over_time" => Ok(MinOverTime),
            "sum_over_time" => Ok(SumOverTime),
            _ => Err(Error::new("Unknown function")),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum FunctionCallArg {
    Expr(Box<Expr>),
    Number(f64),
    String(LabelName),
}

#[derive(Debug, PartialEq)]
pub struct FunctionCall {
    name: FunctionName,
    args: Vec<FunctionCallArg>,
}

impl FunctionCall {
    pub(super) fn new(name: FunctionName, args: Vec<FunctionCallArg>) -> Self {
        use FunctionName::*;

        if name == Vector {
            // TODO: assert args[0] is number
            assert_eq!(args.len(), 1)
        }
        // TODO: check all functions if name != Vector

        Self { name, args }
    }

    #[inline]
    pub fn function_name(&self) -> FunctionName {
        self.name
    }

    pub fn args(self) -> Vec<FunctionCallArg> {
        self.args
    }

    /// An assumption here is that a function has a max one inner expression.
    #[inline]
    pub fn expr(&self) -> Option<&Expr> {
        for arg in self.args.iter() {
            if let FunctionCallArg::Expr(expr) = arg {
                return Some(expr.as_ref());
            }
        }
        None
    }
}

#[derive(Debug, PartialEq)]
pub struct VectorSelector {
    matchers: Vec<LabelMatcher>,
    duration: Option<Duration>,
}

impl VectorSelector {
    pub fn new<S>(
        name: Option<S>,
        mut matchers: Vec<LabelMatcher>,
        duration: Option<Duration>,
    ) -> Result<Self>
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

        Ok(Self { matchers, duration })
    }

    #[inline]
    pub fn matchers(&self) -> &Vec<LabelMatcher> {
        &self.matchers
    }

    #[inline]
    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }
}
