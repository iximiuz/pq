use super::super::line::LineReader;
use super::decoder::{Decoded, Decoder};
use crate::error::Result;

#[derive(Debug)]
pub struct Entry(pub usize, pub Decoded);

pub struct EntryReader {
    inner: Box<dyn LineReader>,
    decoder: Box<dyn Decoder>,
    line_no: usize,
    verbose: bool,
}

impl EntryReader {
    pub fn new(inner: Box<dyn LineReader>, decoder: Box<dyn Decoder>) -> Self {
        Self {
            inner,
            decoder,
            line_no: 0,
            verbose: false,
        }
    }
}

impl std::iter::Iterator for EntryReader {
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut buf = Vec::new();
            match self.inner.read(&mut buf) {
                Ok(0) => return None, // EOF
                Ok(_) => (),
                Err(e) => {
                    return Some(Err(("input reader failed", e).into()));
                }
            };

            self.line_no += 1;

            match self.decoder.decode(&mut buf) {
                Ok(decoded) => return Some(Ok(Entry(self.line_no, decoded))),
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
