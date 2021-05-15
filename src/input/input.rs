use super::decoder::{Decoder, Record};
use super::reader::Reader;
use crate::error::Result;

pub struct Input {
    reader: Box<dyn Reader>,
    decoder: Box<dyn Decoder>,
}

impl Input {
    pub fn new(reader: Box<dyn Reader>, decoder: Box<dyn Decoder>) -> Self {
        Self { reader, decoder }
    }

    pub fn take_one(&mut self) -> Result<Option<Record>> {
        loop {
            let mut buf = Vec::new();
            let n = self
                .reader
                .read(&mut buf)
                .map_err(|e| ("reader failed with error", e))?;
            if n == 0 {
                return Ok(None);
            }

            match self.decoder.decode(&mut buf) {
                Ok(record) => return Ok(Some(record)),
                Err(err) => eprintln!(
                    "Line decoding failed.\nError: {}\nLine: {}",
                    err,
                    String::from_utf8_lossy(&buf),
                ),
            }
        }
    }
}
