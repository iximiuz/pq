use crate::error::{Error, Result};
use crate::utils::parse::{IResult, Span};

#[derive(Debug)]
pub struct AST {
    pub parser: Parser,
    pub mapper: Option<Mapper>,
    pub query: Option<String>,
    pub formatter: Option<Formatter>,
}

#[derive(Debug)]
pub enum Parser {
    Regex(RegexParser),
    JSON,
    CSV(CSVParser),
    Nginx,
    Apache,
    Envoy,
}

#[derive(Debug)]
pub struct RegexParser {
    pub regex: String,
}

#[derive(Debug)]
pub struct CSVParser {
    pub header: Vec<String>,
    pub separator: String,
}

#[derive(Debug)]
pub struct Mapper {}

#[derive(Debug)]
pub enum Formatter {
    JSON,
    PromMetrics,
    HumanReadable,
}

pub fn parse_program(program: &str) -> Result<AST> {
    let (_, parser) = parser(Span::new(program)).map_err(|e| match e {
        nom::Err::Error(e) | nom::Err::Failure(e) => Error::from(e.message()),
        nom::Err::Incomplete(_) => unreachable!(),
    })?;
    Ok(AST {
        parser,
        mapper: None,
        query: None,
        formatter: None,
    })
}

fn parser(input: Span) -> IResult<Parser> {
    Ok((input, Parser::JSON))
}
