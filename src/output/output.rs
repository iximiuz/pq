use std::io::Write;

use super::encoder::Encoder;
use super::writer::Writer;
use crate::engine::Value;
use crate::error::Result;

pub struct Output<W> {
    writer: Box<dyn Writer<W>>,
    encoder: Box<dyn Encoder>,
}

impl<W: Write> Output<W> {
    pub fn new(writer: Box<dyn Writer<W>>, encoder: Box<dyn Encoder>) -> Self {
        Self { writer, encoder }
    }

    pub fn write(&mut self, value: &Value) -> Result<()> {
        let buf = self.encoder.encode(value)?;

        self.writer
            .write(&buf)
            .map_err(|e| ("writer failed with error {}", e))?;

        Ok(())
    }

    pub fn into_inner(self) -> Box<dyn Writer<W>> {
        self.writer
    }
}
