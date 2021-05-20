use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use super::decoder::{Decoder, Record};
use super::reader::Reader;
use crate::model::types::Timestamp;

pub struct Input {
    reader: Box<dyn Reader>,
    decoder: Box<dyn Decoder>,
    cursors: Vec<Weak<Cursor>>,
}

// TODO: implement peakable cursors via multi-threaded reader
//         - a separate thread calls self.reader.read() and puts the record to an internal queue
//           (with some back-pressure mechanism)
//         - main thread takes values only from the queue
//         - if the queue is empty, peak returns None immediately

impl Input {
    pub fn new(reader: Box<dyn Reader>, decoder: Box<dyn Decoder>) -> Self {
        Self {
            reader,
            decoder,
            cursors: vec![],
        }
    }

    pub fn cursor(input: Rc<RefCell<Self>>) -> Rc<Cursor> {
        let cursor = Rc::new(Cursor::new(Rc::clone(&input)));
        input.borrow_mut().cursors.push(Rc::downgrade(&cursor));
        cursor
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

                for weak_cursor in self.cursors.iter_mut() {
                    if let Some(cursor) = weak_cursor.upgrade() {
                        cursor.buffer.borrow_mut().push(sample.clone());
                    }
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
    input: Rc<RefCell<Input>>,
    buffer: RefCell<Vec<Rc<Sample>>>,
}

impl Cursor {
    fn new(input: Rc<RefCell<Input>>) -> Self {
        Cursor {
            input,
            buffer: RefCell::new(vec![]),
        }
    }

    pub fn read(&self) -> Option<Rc<Sample>> {
        if self.buffer.borrow().len() == 0 {
            self.input.borrow_mut().refill_cursors();
        }
        self.buffer.borrow_mut().pop()
    }

    // TODO:
    // pub fn peak(&self) -> Option<Rc<Sample>> {}
}
