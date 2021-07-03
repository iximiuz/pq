use std::collections::HashMap;

use super::decoder::{Decoded, Decoder};
use crate::error::Result;

#[derive(Debug)]
pub enum Entry {
    Tuple(usize, Vec<String>),
    Dict(usize, HashMap<String, String>),
}

type LineIter = std::iter::Iterator<Item = Result<Vec<u8>>>;

pub struct Parser {
    inner: Box<dyn LineIter>,
    decoder: Box<dyn Decoder>,
    line_no: usize,
    verbose: bool,
}

impl Parser {
    pub fn new(inner: Box<dyn LineIter>, decoder: Box<dyn Decoder>) -> Self {
        Self {
            inner,
            decoder,
            line_no: 0,
            verbose: false,
        }
    }
}

impl std::iter::Iterator for Parser {
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = match self.inner.next() {
                Ok(line) => line,
                None => return None, // EOF
                Err(e) => {
                    return Some(Err(("input reader failed", e).into()));
                }
            };

            self.line_no += 1;

            match self.decoder.decode(&line) {
                Ok(Decoded::Tuple(v)) => return Some(Ok(Entry::Tuple(self.line_no, v))),
                Ok(Decoded::Dict(v)) => return Some(Ok(Entry::Dict(self.line_no, v))),
                Err(err) => {
                    if self.verbose {
                        eprintln!(
                            "Line decoding failed.\nError: {}\nLine: {}",
                            err,
                            String::from_utf8_lossy(&buf),
                        );
                    }
                    continue;
                }
            }
        }
    }
}
