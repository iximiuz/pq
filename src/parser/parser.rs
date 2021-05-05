use super::ast::*;
use super::error::ParseError;
use super::result::Span;
use super::vector::*;
use crate::error::{Error, Result};

pub fn parse_query(input: &str) -> Result<AST> {
    let (rest, m) =
        vector_selector(Span::new(input)).map_err(|e| Error::from(ParseError::from(e)))?;
    assert!(rest.len() == 0);
    Ok(AST::new(NodeKind::VectorSelector(m)))
}
