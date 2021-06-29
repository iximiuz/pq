use std::io::{self, BufRead};

pub trait LineReader {
    fn read(&mut self, buf: &mut Vec<u8>) -> io::Result<usize>;
}

pub struct DelimReader<R> {
    inner: R,
    delim: u8,
}

impl<R: BufRead> DelimReader<R> {
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

impl<R: BufRead> LineReader for DelimReader<R> {
    fn read(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.inner.read_until(self.delim, buf)
    }
}
