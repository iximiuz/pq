use std::collections::HashMap;
use std::rc::{Rc, Weak};

use super::decoder::{Decoder, Record};
use super::reader::Reader;
use crate::error::Result;
use crate::model::types::Timestamp;

pub struct Input<'a> {
    reader: Box<dyn Reader>,
    decoder: Box<dyn Decoder>,
    cursors: Vec<Weak<Cursor<'a>>>,
}

impl<'a> Input<'a> {
    pub fn new(reader: Box<dyn Reader>, decoder: Box<dyn Decoder>) -> Self {
        Self {
            reader,
            decoder,
            cursors: vec![],
        }
    }

    pub fn cursor(&'a mut self) -> Rc<Cursor> {
        let cursor = Rc::new(Cursor::new(self));
        cursor
    }

    fn refill_cursors(&mut self) -> bool {
        loop {
            let mut buf = Vec::new();
            match self.reader.read(&mut buf) {
                Err(e) => {
                    eprintln!("reader failed with error {}", e);
                    return false;
                }
                Ok(0) => return false,
                Ok(_) => (),
            };

            match self.decoder.decode(&mut buf) {
                Ok(_) => return true,
                Err(err) => eprintln!(
                    "Line decoding failed.\nError: {}\nLine: {}",
                    err,
                    String::from_utf8_lossy(&buf),
                ),
            }
        }
    }
}

#[derive(Debug)]
pub struct Sample {
    name: String,
    value: f64,
    timestamp: Timestamp,
    labels: HashMap<String, String>,
}

impl Sample {
    pub fn label(&self, name: &str) -> Option<&String> {
        match name {
            "__name__" => Some(&self.name),
            _ => self.labels.get(name),
        }
    }
}

pub struct Cursor<'a> {
    input: &'a mut Input<'a>,
    buffer: Vec<Rc<Sample>>,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a mut Input<'a>) -> Self {
        Cursor {
            input,
            buffer: vec![],
        }
    }
}

// TODO:
// impl Drop for Cursor {
//     fn drop(&mut self) {
//         unsafe {
//             drop_in_place(self.ptr.as_ptr());
//             let c: NonNull<T> = self.ptr.into();
//             Global.deallocate(c.cast(), Layout::new::<T>())
//         }
//     }
// }

impl<'a> std::iter::Iterator for Cursor<'a> {
    type Item = Rc<Sample>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer.len() == 0 && !self.input.refill_cursors() {
            return None;
        }
        self.buffer.pop()
    }
}
