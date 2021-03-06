use std::cell::RefCell;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::format::{Formatter, HumanReadableFormatter, JSONFormatter, PromApiFormatter, Value};
use crate::output::Writer;
use crate::parse::{
    Decoder, DecodingStrategy, JSONDecodingStrategy, Mapper, RegexDecodingStrategy,
};
use crate::program::{self, parse_program};
use crate::query::QueryEvaluator;
use crate::utils::time::TimeRange;

type LineIter = Box<dyn std::iter::Iterator<Item = Result<(usize, Vec<u8>)>>>;

pub struct RunnerOptions {
    verbose: bool,
    interactive: bool,
    range: Option<TimeRange>,
    interval: Option<Duration>,
    lookback: Option<Duration>,
}

impl RunnerOptions {
    pub fn new(
        verbose: bool,
        interactive: bool,
        range: Option<TimeRange>,
        interval: Option<Duration>,
        lookback: Option<Duration>,
    ) -> RunnerOptions {
        RunnerOptions {
            verbose,
            interactive,
            range,
            interval,
            lookback,
        }
    }
}

pub struct Runner {
    producer: Producer,
    consumer: Consumer,
    verbose: bool,
}

impl Runner {
    pub fn new(
        program: &str,
        reader: LineIter,
        writer: Box<dyn Writer>,
        options: RunnerOptions,
    ) -> Result<Self> {
        let RunnerOptions {
            verbose,
            interactive,
            range,
            interval,
            lookback,
        } = options;

        let ast = parse_program(program)?;

        let decoding: Box<dyn DecodingStrategy> = match ast.decoder {
            program::Decoder::JSON => Box::new(JSONDecodingStrategy::default()),
            program::Decoder::Regex { regex } => Box::new(RegexDecodingStrategy::new(&regex)?),
        };
        let decoder = Decoder::new(reader, decoding);

        if let Some(program::Formatter::HumanReadable) = ast.formatter {
            if interactive {
                return Err(Error::new(
                    "interactive mode only supported if no formatter was specified",
                ));
            }
        }

        let formatter: Box<dyn Formatter> = match ast.formatter {
            Some(program::Formatter::HumanReadable) => {
                Box::new(HumanReadableFormatter::new(verbose, interactive))
            }
            Some(program::Formatter::JSON) => Box::new(JSONFormatter::new(verbose)),
            Some(program::Formatter::PromAPI) => Box::new(PromApiFormatter::default()),
            None => Box::new(HumanReadableFormatter::new(verbose, interactive)),
        };

        let consumer = Consumer::new(writer, formatter);

        let range = range.unwrap_or_else(TimeRange::infinity);

        let mapper = match ast.mapper {
            Some(mapper) => Mapper::new(Box::new(decoder), mapper, Some(range)),
            None => {
                if ast.query.is_some() {
                    return Err(Error::new(
                        "'select' without 'map' is not supported for this parser",
                    ));
                }

                return Ok(Self {
                    producer: Producer::Decoder(RefCell::new(decoder)),
                    consumer,
                    verbose,
                });
            }
        };

        let query = match ast.query {
            Some(query) => query,
            None => {
                return Ok(Self {
                    producer: Producer::Mapper(RefCell::new(mapper)),
                    consumer,
                    verbose,
                });
            }
        };

        Ok(Self {
            producer: Producer::Querier(RefCell::new(QueryEvaluator::new(
                query,
                Box::new(mapper),
                interval,
                lookback,
                range.start(),
                verbose, // TODO: remove it when proper error propagation is in place.
            )?)),
            consumer,
            verbose,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let value = match &self.producer {
                Producer::Decoder(decoder) => match decoder.borrow_mut().next() {
                    Some(Ok(entry)) => Value::Entry(entry),
                    Some(Err(e)) => {
                        if self.verbose {
                            eprintln!("{}", e);
                        }
                        continue;
                    }
                    None => break,
                },
                Producer::Mapper(mapper) => match mapper.borrow_mut().next() {
                    Some(Ok(record)) => Value::Record(record),
                    Some(Err(e)) => {
                        if self.verbose {
                            eprintln!("{}", e);
                        }
                        continue;
                    }
                    None => break,
                },
                Producer::Querier(querier) => match querier.borrow_mut().next() {
                    Some(Ok(value)) => Value::QueryValue(value),
                    Some(Err(e)) => {
                        if self.verbose {
                            eprintln!("{}", e);
                        }
                        continue;
                    }
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
        if !buf.is_empty() {
            self.writer
                .write(&buf)
                .map_err(|e| ("writer failed with error {}", e))?;
        }

        Ok(())
    }
}
