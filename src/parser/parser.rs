use super::ast::*;
use super::error::ParseError;
use super::result::{ParseResult, Span};
use super::vector::*;
use crate::error::{Error, Result};

pub fn parse_query(input: &str) -> Result<AST> {
    let (rest, m) = match vector_selector(Span::new(input)) {
        Ok((r, ParseResult::Complete(m))) => (r, m),
        Ok((unexpected, ParseResult::Partial(wherein, expected))) => {
            return Err(Error::from(ParseError::new(
                *unexpected,
                (unexpected.location_line(), unexpected.location_offset()),
                wherein,
                expected,
            )))
        }
        Err(e) => return Err(Error::from(ParseError::from(e))),
    };
    assert!(rest.len() == 0);
    Ok(AST::new(NodeKind::VectorSelector(m)))
}
