use std::io::{self, Write};

pub trait Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<()>;
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

    pub fn new_with_delimiter(inner: W, delim: u8) -> Self {
        Self { inner, delim }
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Writer for LineWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.write_all(buf)?;
        self.inner.write_all(&[self.delim])
    }
}
