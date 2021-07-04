use std::collections::HashMap;

use super::strategy::{DecodingResult, DecodingStrategy};
use crate::error::Result;

#[derive(Debug)]
pub enum Entry {
    Tuple(usize, Vec<String>),
    Dict(usize, HashMap<String, String>),
}

impl Entry {
    pub fn line_no(&self) -> usize {
        match self {
            Entry::Tuple(line_no, _) => *line_no,
            Entry::Dict(line_no, _) => *line_no,
        }
    }
}

type LineIter = Box<dyn std::iter::Iterator<Item = Result<(usize, Vec<u8>)>>>;

pub struct Decoder {
    inner: LineIter,
    strategy: Box<dyn DecodingStrategy>,
}

impl Decoder {
    pub fn new(inner: LineIter, strategy: Box<dyn DecodingStrategy>) -> Self {
        Self { inner, strategy }
    }
}

impl std::iter::Iterator for Decoder {
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        let (line_no, line) = match self.inner.next() {
            Some(Ok((line_no, line))) => (line_no, line),
            Some(Err(e)) => return Some(Err(e)),
            None => return None, // EOF
        };

        match self.strategy.decode(&line) {
            Ok(DecodingResult::Tuple(v)) => Some(Ok(Entry::Tuple(line_no, v))),
            Ok(DecodingResult::Dict(v)) => Some(Ok(Entry::Dict(line_no, v))),
            Err(e) => return Some(Err(("line decoding failed", e).into())),
        }
    }
}
