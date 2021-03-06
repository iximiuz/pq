use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

use crate::error::Result;
use crate::model::{Labels, MetricName, SampleValue, Timestamp};
use crate::parse::Record;

#[derive(Debug)]
pub struct Sample {
    value: SampleValue,
    timestamp: Timestamp,
    labels: Labels,
}

impl Sample {
    pub fn new(
        name: MetricName,
        value: SampleValue,
        timestamp: Timestamp,
        mut labels: Labels,
    ) -> Self {
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

pub struct SampleReader {
    records: Box<dyn std::iter::Iterator<Item = Result<Record>>>,
    cursors: Vec<Weak<Cursor>>,
    verbose: bool, // TODO: remove it
}

impl SampleReader {
    pub fn new(
        records: Box<dyn std::iter::Iterator<Item = Result<Record>>>,
        verbose: bool,
    ) -> Self {
        Self {
            records,
            cursors: vec![],
            verbose,
        }
    }

    pub fn cursor(reader: Rc<RefCell<Self>>) -> Rc<Cursor> {
        let cursor = Rc::new(Cursor::new(Rc::clone(&reader)));
        reader.borrow_mut().cursors.push(Rc::downgrade(&cursor));
        cursor
    }

    fn refill_cursors(&mut self) {
        // TODO: optimize - read multiple records at once.
        // TODO: propagate errors.
        loop {
            match self.records.next() {
                Some(Ok(record)) => {
                    let (line_no, timestamp, labels, mut values) = (
                        record.line_no(),
                        record.timestamp(),
                        record.labels(),
                        record.values().clone(),
                    );

                    if let Some(timestamp) = timestamp {
                        // Tiny hack...
                        values.insert("__line__".to_owned(), line_no as SampleValue);

                        for (name, value) in values {
                            let sample =
                                Rc::new(Sample::new(name, value, timestamp, labels.clone()));

                            for weak_cursor in self.cursors.iter_mut() {
                                if let Some(cursor) = weak_cursor.upgrade() {
                                    cursor.buffer.borrow_mut().push_front(sample.clone());
                                }
                            }
                        }

                        break;
                    }
                }
                Some(Err(e)) if self.verbose => {
                    eprintln!("{}", e);
                }
                None => break,
                _ => (),
            }
        }
    }
}

pub struct Cursor {
    reader: Rc<RefCell<SampleReader>>,
    buffer: RefCell<VecDeque<Rc<Sample>>>,
}

impl Cursor {
    fn new(reader: Rc<RefCell<SampleReader>>) -> Self {
        Cursor {
            reader,
            buffer: RefCell::new(VecDeque::new()),
        }
    }

    pub fn read(&self) -> Option<Rc<Sample>> {
        if self.buffer.borrow().len() == 0 {
            self.reader.borrow_mut().refill_cursors();
        }
        self.buffer.borrow_mut().pop_back()
    }
}
