use crate::error::Result;
use crate::utils::parse::{IResult, Span};

#[derive(Debug)]
pub struct AST {
    pub parser: Parser,
    pub mapper: Option<Mapper>,
    pub query: Option<String>,
    pub formatter: Option<Formatter>,
}

pub enum Parser {
    Regex(RegexParser),
    JSON,
    CSV(CSVParser),
    Nginx,
    Apache,
    Envoy,
}

pub struct RegexParser {
    pub regex: String,
}

pub struct CSVParser {
    pub header: Vec<String>,
    pub separator: String,
}

pub struct Mapper {}

pub enum Formatter {
    JSON,
    PromMetrics,
    HumanReadable,
}

pub fn parse_program(program: &str) -> Result<AST> {
    let (_, parser) = parser(Span::new(program))?;
    Ok(AST { parser })
}

fn parser(input: Span) -> IResult<Parser> {
    Ok((input, Parser::JSON))
}
