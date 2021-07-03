use std::io::BufRead;

pub struct LineReader<R> {
    inner: R,
    delim: u8,
}

impl<R: BufRead> LineReader<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            delim: b'\n',
        }
    }

    pub fn with_delimiter(inner: R, delim: u8) -> Self {
        Self { inner, delim }
    }
}

impl<R: BufRead> std::iter::Iterator for LineReader<R> {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = Vec::new();
        match self.inner.read_until(self.delim, buf) {
            Ok(0) => None,
            Ok(_) => Some(Ok(buf)),
            Err(e) => Some(Err(e)),
        }
    }
}
