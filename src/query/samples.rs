use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

use crate::input::Record;
use crate::model::Sample;

pub struct SampleReader {
    records: Box<dyn std::iter::Iterator<Item = Record>>,
    cursors: Vec<Weak<Cursor>>,
}

impl SampleReader {
    pub fn new(records: Box<dyn std::iter::Iterator<Item = Record>>) -> Self {
        Self {
            records,
            cursors: vec![],
        }
    }

    pub fn cursor(reader: Rc<RefCell<Self>>) -> Rc<Cursor> {
        let cursor = Rc::new(Cursor::new(Rc::clone(&reader)));
        reader.borrow_mut().cursors.push(Rc::downgrade(&cursor));
        cursor
    }

    fn refill_cursors(&mut self) {
        // TODO: optimize - read multiple records at once.
        if let Some(Record(timestamp, labels, values)) = self.records.next() {
            for (name, value) in values {
                let sample = Rc::new(Sample::new(name, value, timestamp, labels.clone()));

                for weak_cursor in self.cursors.iter_mut() {
                    if let Some(cursor) = weak_cursor.upgrade() {
                        cursor.buffer.borrow_mut().push_front(sample.clone());
                    }
                }
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
