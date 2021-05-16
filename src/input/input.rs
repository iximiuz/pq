use std::collections::HashMap;
use std::rc::Rc;

use super::decoder::{Decoder, Record};
use super::reader::Reader;
use crate::model::types::Timestamp;

pub struct Input {
    reader: Box<dyn Reader>,
    decoder: Box<dyn Decoder>,
    cursors: Vec<Cursor>,
}

impl Input {
    pub fn new(reader: Box<dyn Reader>, decoder: Box<dyn Decoder>) -> Self {
        Self {
            reader,
            decoder,
            cursors: vec![],
        }
    }

    pub fn cursor<'a>(&'a mut self) -> &'a mut Cursor {
        let cursor = Cursor::new(self);
        self.cursors.push(cursor);
        self.cursors.last_mut().unwrap()
    }

    fn refill_cursors(&mut self) {
        loop {
            let mut buf = Vec::new();
            match self.reader.read(&mut buf) {
                Err(e) => {
                    eprintln!("reader failed with error {}", e);
                    break;
                }
                Ok(0) => break, // EOF
                Ok(_) => (),
            };

            let (timestamp, labels, values) = match self.decoder.decode(&mut buf) {
                Ok(Record(ts, ls, vs)) => (ts, ls, vs),
                Err(err) => {
                    eprintln!(
                        "Line decoding failed.\nError: {}\nLine: {}",
                        err,
                        String::from_utf8_lossy(&buf),
                    );
                    continue;
                }
            };

            for (name, value) in values {
                let sample = Rc::new(Sample {
                    name,
                    value,
                    timestamp,
                    labels: labels.clone(),
                });

                for cursor in self.cursors.iter_mut() {
                    cursor.buffer.push(sample.clone());
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Sample {
    pub name: String,
    pub value: f64,
    pub timestamp: Timestamp,
    pub labels: HashMap<String, String>,
}

impl Sample {
    pub fn label(&self, name: &str) -> Option<&String> {
        match name {
            "__name__" => Some(&self.name),
            _ => self.labels.get(name),
        }
    }
}

pub struct Cursor {
    input: *mut Input,
    buffer: Vec<Rc<Sample>>,
}

impl Cursor {
    fn new(input: *mut Input) -> Self {
        Cursor {
            input,
            buffer: vec![],
        }
    }
}

impl std::iter::Iterator for Cursor {
    type Item = Rc<Sample>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer.len() == 0 {
            unsafe {
                (*self.input).refill_cursors();
            }
        }
        self.buffer.pop()
    }
}
