use std::cell::RefCell;
use std::time::Duration;

use crate::error::Result;
use crate::formatter::Formatter;
use crate::output::{Value, Writer};
use crate::parse::{Entry, Parser};
use crate::program::{self, parse_program};
use crate::query::QueryEvaluator;
use crate::utils::time::TimeRange;

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

type LineIter = Box<dyn std::iter::Iterator<Item = Result<Vec<u8>>>>;

pub struct Runner {
    producer: Producer,
    consumer: Consumer,
}

impl Runner {
    pub fn new(
        program: &str,
        reader: LineIter,
        writer: Box<dyn Writer>,
        range: Option<TimeRange>,
        interval: Option<Duration>,
        lookback: Option<Duration>,
    ) -> Result<Self> {
        // TODO: trim spaces from program.

        let ast = parse_program(program)?;

        let parser = match ast.parser {
            program::Parser::Regex(opt) => Parser::new(RegexDecoder::new(opt.regex)),
            _ => unimplemented!(),
        };

        let formatter = match ast.formatter {
            Some(program::Formatter::JSON) => Formatter::new(),
            _ => unreachable!(),
        };

        let consumer = Consumer::new(writer, formatter);

        let range = range.unwrap_or(TimeRange::infinity());

        let mapper = match ast.mapper {
            Some(mapper) => Mapper::new(parser, range),
            None => {
                return Ok(Self {
                    producer: Producer::Parser(RefCell::new(parser)),
                    consumer,
                });
            }
        };

        let query = match ast.query {
            Some(query) => query,
            None => {
                return Ok(Self {
                    producer: Producer::Mapper(RefCell::new(mapper)),
                    consumer,
                });
            }
        };

        // TODO: make sure matcher has a timestamp match.
        // TODO: compare decoder entry size and matcher pattern size.

        Ok(Self {
            producer: Producer::QueryReader(RefCell::new(QueryEvaluator::new(
                query,
                Box::new(mapper),
                interval,
                lookback,
                range.start(),
            )?)),
            consumer,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            // TODO: incorporate this logic somewhere...
            // if iter_value_kind == ExprValueKind::Scalar {
            //     break;
            // }

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
                Producer::QueryReader(qreader) => match qreader.borrow_mut().next() {
                    Some(Ok(value)) => Encodable::QueryValue(value),
                    Some(Err(e)) => return Err(e),
                    None => break,
                },
            };
            self.consumer.write(&encodable)?;
        }
        Ok(())
    }
}

enum Producer {
    EntryReader(RefCell<EntryReader>),
    RecordReader(RefCell<RecordReader>),
    QueryReader(RefCell<QueryEvaluator>),
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
