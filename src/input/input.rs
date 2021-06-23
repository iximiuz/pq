use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

use super::decoder::{Decoder, Record};
use super::reader::Reader;
use crate::model::types::{Labels, MetricName, SampleValue, Timestamp};

pub struct Input {
    reader: Box<dyn Reader>,
    decoder: Box<dyn Decoder>,
    line_no: usize,
    cursors: Vec<Weak<Cursor>>,
}

// TODO: implement multi-threaded reader
//         - a separate thread calls self.reader.read() and puts the record to an internal queue
//           (with some backpressure mechanism)
//         - main thread takes values only from the queue

impl Input {
    pub fn new(reader: Box<dyn Reader>, decoder: Box<dyn Decoder>) -> Self {
        Self {
            reader,
            decoder,
            line_no: 0,
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

            self.line_no += 1;

            let (timestamp, labels, mut values) = match self.decoder.decode(&mut buf) {
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

            // Tiny hack...
            values.insert("__line__".to_owned(), self.line_no as SampleValue);

            for (name, value) in values {
                let sample = Rc::new(Sample::new(name, value, timestamp, labels.clone()));

                for weak_cursor in self.cursors.iter_mut() {
                    if let Some(cursor) = weak_cursor.upgrade() {
                        cursor.buffer.borrow_mut().push_front(sample.clone());
                    }
                }
            }

            // TODO: optimize - read multiple lines at once.
            break;
        }
    }
}

pub struct Cursor {
    input: Rc<RefCell<Input>>,
    buffer: RefCell<VecDeque<Rc<Sample>>>,
}

impl Cursor {
    fn new(input: Rc<RefCell<Input>>) -> Self {
        Cursor {
            input,
            buffer: RefCell::new(VecDeque::new()),
        }
    }

    pub fn read(&self) -> Option<Rc<Sample>> {
        if self.buffer.borrow().len() == 0 {
            self.input.borrow_mut().refill_cursors();
        }
        self.buffer.borrow_mut().pop_back()
    }
}

#[derive(Debug)]
pub struct Sample {
    value: SampleValue,
    timestamp: Timestamp,
    labels: Labels,
}

impl Sample {
    fn new(name: MetricName, value: SampleValue, timestamp: Timestamp, mut labels: Labels) -> Self {
        labels.insert("__name__".into(), name);
        Self {
            value,
            timestamp,
            labels,
        }
    }

    #[inline]
    pub fn value(&self) -> SampleValue {
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

    pub fn label(&self, name: &str) -> Option<&MetricName> {
        self.labels.get(name)
    }
}
