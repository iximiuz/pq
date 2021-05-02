use super::ast::*;
use super::result::Span;
use super::vector::*;
use crate::error::Result;

pub fn parse_query(input: &str) -> Result<AST> {
    let (rest, m) =
        vector_selector(Span::new(input)).map_err(|e| ("couldn't parse PromQL query", e))?;
    assert!(rest.len() == 0);
    Ok(AST::new(NodeKind::VectorSelector(m)))
}
