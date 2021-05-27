use std::cell::RefCell;
use std::rc::{Rc, Weak};

use super::decoder::{Decoder, Record};
use super::reader::Reader;
use crate::model::types::{Labels, Timestamp, Value};

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
                let sample = Rc::new(Sample::new(name, value, timestamp, labels.clone()));

                for weak_cursor in self.cursors.iter_mut() {
                    if let Some(cursor) = weak_cursor.upgrade() {
                        cursor.buffer.borrow_mut().push(sample.clone());
                    }
                }
            }
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

#[derive(Debug)]
pub struct Sample {
    value: Value,
    timestamp: Timestamp,
    labels: Labels,
}

impl Sample {
    fn new(name: String, value: Value, timestamp: Timestamp, mut labels: Labels) -> Self {
        labels.insert("__name__".into(), name);
        Self {
            value,
            timestamp,
            labels,
        }
    }

    #[inline]
    pub fn value(&self) -> Value {
        self.value
    }

    #[inline]
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    #[inline]
    pub fn labels(&self) -> &Labels {
        &self.labels
    }

    pub fn label(&self, name: &str) -> Option<&String> {
        self.labels.get(name)
    }
}
