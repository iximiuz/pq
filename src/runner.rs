use std::cell::RefCell;

// use crate::common::time::TimeRange;
use crate::error::Result;
use crate::input::{parse_matcher, Decoder, EntryReader, LineReader, RecordMatcher, RecordReader};
use crate::output::{Encodable, Encoder, Writer};

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
//             -> Encodable(Entry | Record | ExprValue)
//               -> Line or Multiline (loosely, a String)
//                 -> stdout

pub struct Runner {
    producer: Producer,
    consumer: Consumer,
    // range: TimeRange,
}

impl Runner {
    pub fn new(
        reader: Box<dyn LineReader>,
        decoder: Box<dyn Decoder>,
        encoder: Box<dyn Encoder>,
        writer: Box<dyn Writer>,
        pattern: Option<&str>,
        // query: Option<String>,
        // range: Option<TimeRange>,
    ) -> Result<Self> {
        let consumer = Consumer::new(writer, encoder);
        let ereader = EntryReader::new(reader, decoder);

        if let Some(pattern) = pattern {
            let rreader = RecordReader::new(Box::new(ereader), parse_matcher(pattern)?);

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

            return Ok(Self {
                producer: Producer::RecordReader(RefCell::new(rreader)),
                consumer,
                // range: range.unwrap_or(TimeRange::infinity()),
            });
        }

        Ok(Self {
            producer: Producer::EntryReader(RefCell::new(ereader)),
            consumer,
            // range: range.unwrap_or(TimeRange::infinity()),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let encodable = match &self.producer {
                Producer::EntryReader(ereader) => match ereader.borrow_mut().next() {
                    Some(Ok(entry)) => Encodable::Entry(entry),
                    Some(Err(e)) => return Err(e),
                    None => break,
                },
                Producer::RecordReader(rreader) => match rreader.borrow_mut().next() {
                    Some(Ok(record)) => Encodable::Record(record),
                    Some(Err(e)) => return Err(e),
                    None => break,
                },
            };
            self.consumer.write(&encodable)?;
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

    pub fn write(&mut self, value: &Encodable) -> Result<()> {
        let buf = self.encoder.encode(value)?;

        self.writer
            .write(&buf)
            .map_err(|e| ("writer failed with error {}", e))?;

        Ok(())
    }
}
