use std::io::{self, BufRead};

pub trait Reader {
    fn read(&mut self, buf: &mut Vec<u8>) -> io::Result<usize>;
}

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

impl<R: BufRead> Reader for LineReader<R> {
    fn read(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.inner.read_until(self.delim, buf)
    }
}
