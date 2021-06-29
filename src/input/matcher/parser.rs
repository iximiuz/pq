// use super::list::ListMatcher;
use super::matcher::RecordMatcher;
use crate::common::parser::Span;
use crate::error::Result;

pub fn parse_matcher(input: &str) -> Result<Box<dyn RecordMatcher>> {
    unimplemented!();
}
