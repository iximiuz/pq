use super::encoder::Encoder;
use super::writer::Writer;
use crate::engine::ValueKind;
use crate::error::Result;

pub struct Output {
    writer: Box<dyn Writer>,
    encoder: Box<dyn Encoder>,
}

impl Output {
    pub fn new(writer: Box<dyn Writer>, encoder: Box<dyn Encoder>) -> Self {
        Self { writer, encoder }
    }

    pub fn write(&mut self, value: &ValueKind) -> Result<()> {
        let buf = self.encoder.encode(value)?;

        self.writer
            .write(&buf)
            .map_err(|e| ("writer failed with error {}", e))?;

        Ok(())
    }
}
