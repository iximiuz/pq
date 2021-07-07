use std::io::BufRead;

use crate::error::Result;

pub struct LineReader<R> {
    inner: R,
    delim: u8,
    line_no: usize,
}

impl<R: BufRead> LineReader<R> {
    pub fn new(inner: R) -> Self {
        Self::with_delimiter(inner, b'\n')
    }

    pub fn with_delimiter(inner: R, delim: u8) -> Self {
        Self {
            inner,
            delim,
            line_no: 0,
        }
    }
}

impl<R: BufRead> std::iter::Iterator for LineReader<R> {
    type Item = Result<(usize, Vec<u8>)>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = Vec::new();
        self.line_no += 1;

        match self.inner.read_until(self.delim, &mut buf) {
            Ok(0) => None,
            Ok(_) => Some(Ok((self.line_no, buf))),
            Err(e) => Some(Err(("input reader failed", e).into())),
        }
    }
}
