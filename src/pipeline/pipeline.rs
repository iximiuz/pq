use std::cell::RefCell;

// use crate::common::time::TimeRange;
use crate::decoder::{Decoder, Entry};
use crate::encoder::{Encoder, Outry};
use crate::error::Result;
use crate::input::Reader;
use crate::model::Record;
use crate::output::Writer;

// (Reader -> Decoder [-> Matcher [-> Querier]]) -> (Encoder -> Writer)
//                 producer                              consumer
//
// Reader  == stdin [, line separator]  ->  Iterator<Result<String>>
// Decoder == Iterator<String>          ->  Iterator<Result<Entry>>
// Matcher == Iterator<Entry>           ->  Iterator<Result<Record>>
// Querier == Iterator<Record>          ->  Iterator<ExprValue>
// Encoder == Iterator<ExprValue>       ->  Iterator<Result<String>>
// Writer  == Iterator<String>          ->  stdout
//
// stdin
//   -> Line or Multiline (loosely, a String)
//     -> Entry(Vec | Dict)
//       -> Record(timestamp, labels, values)
//         -> Sample(timestamp, labels, value)
//           -> ExprValue(InstantVector | RangeVector | Scalar)
//             -> Line or Multiline (loosely, a String)
//               -> stdout

pub struct Pipeline {
    producer: Producer,
    consumer: Consumer,
    // range: TimeRange,
}

impl Pipeline {
    pub fn new(
        reader: Box<dyn Reader>,
        decoder: Box<dyn Decoder>,
        encoder: Box<dyn Encoder>,
        writer: Box<dyn Writer>,
        pattern: Option<String>,
        // query: Option<String>,
        // range: Option<TimeRange>,
    ) -> Self {
        let consumer = Consumer::new(writer, encoder);
        let ereader = EntryReader::new(reader, decoder);

        if let Some(pattern) = pattern {
            let rreader = RecordReader::new(Box::new(ereader));

            // TODO:
            // if let Some(query) = query {
            //     TODO: make sure matcher has a timestamp match.
            //
            //     return Self {
            //         producer: Producer::ExprValueReader(QueryExecutor::new(rreader))
            //         consumer,
            //     }
            // }

            // TODO: compare decoder entry size and matcher pattern size.

            return Self {
                producer: Producer::RecordReader(RefCell::new(rreader)),
                consumer,
                // range: range.unwrap_or(TimeRange::infinity()),
            };
        }

        Self {
            producer: Producer::EntryReader(RefCell::new(ereader)),
            consumer,
            // range: range.unwrap_or(TimeRange::infinity()),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let outry = match &self.producer {
                Producer::EntryReader(ereader) => match ereader.borrow_mut().next() {
                    Some(Ok((entry, line_no))) => Outry::Entry(entry, line_no),
                    Some(Err(e)) => return Err(e),
                    None => break,
                },
                Producer::RecordReader(rreader) => match rreader.borrow_mut().next() {
                    Some(Ok(record)) => Outry::Record(record),
                    Some(Err(e)) => return Err(e),
                    None => break,
                },
            };
            self.consumer.write(&outry)?;
        }
        Ok(())
    }

    // fn execute(&self, query: AST) -> Result<()> {
    //     // println!("Executor::execute {:#?}", query);

    //     let iter = self.create_value_iter(query.root);
    //     let iter_value_kind = iter.value_kind();
    //     for value in iter {
    //         self.output.borrow_mut().write(&value)?;
    //         // TODO: if value iter is scalar, we need to wrap it into
    //         //       something that would produce a (timestamp, scalar) tuples
    //         //       instead.
    //         if iter_value_kind == ExprValueKind::Scalar {
    //             break;
    //         }
    //     }
    // }
}

struct EntryReader {
    reader: Box<dyn Reader>,
    decoder: Box<dyn Decoder>,
    line_no: usize,
    verbose: bool,
}

impl EntryReader {
    fn new(reader: Box<dyn Reader>, decoder: Box<dyn Decoder>) -> Self {
        Self {
            reader,
            decoder,
            line_no: 0,
            verbose: false,
        }
    }
}

impl std::iter::Iterator for EntryReader {
    type Item = Result<(Entry, usize)>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut buf = Vec::new();
            match self.reader.read(&mut buf) {
                Ok(_) => (),
                Ok(0) => return None, // EOF
                Err(e) => {
                    return Some(Err(("reader failed", e).into()));
                }
            };

            self.line_no += 1;

            match self.decoder.decode(&mut buf) {
                Ok(entry) => return Some(Ok((entry, self.line_no))),
                Err(err) => {
                    if self.verbose {
                        eprintln!(
                            "Line decoding failed.\nError: {}\nLine: {}",
                            err,
                            String::from_utf8_lossy(&buf),
                        );
                    }
                    continue;
                }
            }
        }
    }
}

struct RecordReader {
    entries: Box<dyn std::iter::Iterator<Item = Result<(Entry, usize)>>>,
    // matcher: Box<dyn Matcher>,
}

impl RecordReader {
    fn new(entries: Box<dyn std::iter::Iterator<Item = Result<(Entry, usize)>>>) -> Self {
        Self { entries }
    }
}

impl std::iter::Iterator for RecordReader {
    type Item = Result<Record>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (entry, line) = match self.entries.next() {
                Some(Ok((entry, line))) => (entry, line),
                Some(Err(e)) => {
                    return Some(Err(("reader failed", e).into()));
                }
                None => return None, // EOF
            };

            // TODO:
            // Tiny hack...
            // values.insert("__line__".to_owned(), self.line_no as SampleValue);

            // if sample.timestamp() > self.last_instant.unwrap_or(Timestamp::MAX) {
            //     // Input not really drained, but we've seen enough.
            //     return None;
            // }

            return None;
        }
    }
}

enum Producer {
    EntryReader(RefCell<EntryReader>),
    RecordReader(RefCell<RecordReader>),
}

struct Consumer {
    writer: Box<dyn Writer>,
    encoder: Box<dyn Encoder>,
}

impl Consumer {
    fn new(writer: Box<dyn Writer>, encoder: Box<dyn Encoder>) -> Self {
        Self { writer, encoder }
    }

    pub fn write(&mut self, value: &Outry) -> Result<()> {
        let buf = self.encoder.encode(value)?;

        self.writer
            .write(&buf)
            .map_err(|e| ("writer failed with error {}", e))?;

        Ok(())
    }
}
