use std::cell::RefCell;
use std::time::Duration;

use crate::error::Result;
use crate::format::{Formatter, HumanReadableFormatter, Value};
use crate::output::Writer;
use crate::parse::{Decoder, Mapper, RegexDecodingStrategy};
use crate::program::{self, parse_program};
use crate::query::QueryEvaluator;
use crate::utils::time::TimeRange;

type LineIter = Box<dyn std::iter::Iterator<Item = Result<(usize, Vec<u8>)>>>;

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
        let ast = parse_program(program)?;

        let decoder = match ast.decoder {
            program::Decoder::Regex { regex } => {
                Decoder::new(reader, Box::new(RegexDecodingStrategy::new(&regex)?))
            }
            _ => unimplemented!(),
        };

        let formatter = match ast.formatter {
            None | Some(program::Formatter::HumanReadable) => HumanReadableFormatter::new(),
            _ => unreachable!(),
        };

        let consumer = Consumer::new(writer, Box::new(formatter));

        let range = range.unwrap_or(TimeRange::infinity());

        let mapper = match ast.mapper {
            Some(mapper) => Mapper::new(Box::new(decoder), Some(range)),
            None => {
                return Ok(Self {
                    producer: Producer::Decoder(RefCell::new(decoder)),
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
            producer: Producer::Querier(RefCell::new(QueryEvaluator::new(
                &query,
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

            let value = match &self.producer {
                Producer::Decoder(decoder) => match decoder.borrow_mut().next() {
                    Some(Ok(entry)) => Value::Entry(entry),
                    Some(Err(e)) => return Err(e),
                    None => break,
                },
                Producer::Mapper(mapper) => match mapper.borrow_mut().next() {
                    Some(Ok(record)) => Value::Record(record),
                    Some(Err(e)) => return Err(e),
                    None => break,
                },
                Producer::Querier(querier) => match querier.borrow_mut().next() {
                    Some(Ok(value)) => Value::QueryValue(value),
                    Some(Err(e)) => return Err(e),
                    None => break,
                },
            };
            self.consumer.write(&value)?;
        }
        Ok(())
    }
}

enum Producer {
    Decoder(RefCell<Decoder>),
    Mapper(RefCell<Mapper>),
    Querier(RefCell<QueryEvaluator>),
}

struct Consumer {
    writer: Box<dyn Writer>,
    formatter: Box<dyn Formatter>,
}

impl Consumer {
    fn new(writer: Box<dyn Writer>, formatter: Box<dyn Formatter>) -> Self {
        Self { writer, formatter }
    }

    pub fn write(&mut self, value: &Value) -> Result<()> {
        let buf = self.formatter.format(value)?;

        self.writer
            .write(&buf)
            .map_err(|e| ("writer failed with error {}", e))?;

        Ok(())
    }
}
