use std::collections::HashMap;

use super::strategy::{DecodingResult, DecodingStrategy};
use crate::error::Result;

#[derive(Debug)]
pub enum Entry {
    Tuple(usize, Vec<String>),
    Dict(usize, HashMap<String, String>),
}

type LineIter = Box<dyn std::iter::Iterator<Item = Result<(usize, Vec<u8>)>>>;

pub struct Decoder {
    inner: LineIter,
    strategy: Box<dyn DecodingStrategy>,
    verbose: bool,
}

impl Decoder {
    pub fn new(inner: LineIter, strategy: Box<dyn DecodingStrategy>) -> Self {
        Self {
            inner,
            strategy,
            verbose: false,
        }
    }
}

impl std::iter::Iterator for Decoder {
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (line_no, line) = match self.inner.next() {
                Some(Ok((line_no, line))) => (line_no, line),
                Some(Err(e)) => return Some(Err(e)),
                None => return None, // EOF
            };

            match self.strategy.decode(&line) {
                Ok(DecodingResult::Tuple(v)) => return Some(Ok(Entry::Tuple(line_no, v))),
                Ok(DecodingResult::Dict(v)) => return Some(Ok(Entry::Dict(line_no, v))),
                Err(err) => {
                    if self.verbose {
                        eprintln!(
                            "Line decoding failed.\nError: {}\nLine: {}",
                            err,
                            String::from_utf8_lossy(&line),
                        );
                    }
                    continue;
                }
            }
        }
    }
}
