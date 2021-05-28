use std::io::{self, Write};

pub trait Writer<W: Write> {
    fn write(&mut self, buf: &Vec<u8>) -> io::Result<()>;
    fn into_inner(self) -> W;
}

pub struct LineWriter<W> {
    inner: W,
    delim: u8,
}

impl<W: Write> LineWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            delim: b'\n',
        }
    }

    pub fn with_delimiter(inner: W, delim: u8) -> Self {
        Self { inner, delim }
    }
}

impl<W: Write> Writer<W> for LineWriter<W> {
    fn write(&mut self, buf: &Vec<u8>) -> io::Result<()> {
        self.inner.write_all(buf)?;
        self.inner.write_all(&[self.delim])
    }

    fn into_inner(self) -> W {
        self.inner
    }
}
