use std::io::{self, BufReader};

use structopt::StructOpt;

use pq::cliopt::CliOpt;
use pq::common::time::TimeRange;
use pq::engine::Executor;
use pq::input::{decoder::RegexDecoder, reader::LineReader, Input};
use pq::output::{
    encoder::{HumanReadableEncoder, PromApiEncoder},
    writer::LineWriter,
    Output,
};
use pq::parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = CliOpt::from_args();

    let input = Input::new(
        Box::new(LineReader::new(BufReader::new(io::stdin()))),
        Box::new(RegexDecoder::new(
            &opt.decode,
            opt.timestamp,
            opt.labels,
            opt.metrics,
        )?),
        opt.verbose,
    );

    let output = Output::new(
        Box::new(LineWriter::new(io::stdout())),
        match opt.encode {
            None => Box::new(PromApiEncoder::new()),
            Some(e) if e == "h" => Box::new(HumanReadableEncoder::new()),
            _ => unimplemented!(),
        },
    );

    let exctr = Executor::new(
        input,
        output,
        Some(TimeRange::new(opt.since, opt.until)?),
        opt.interval,
        opt.lookback,
    );

    let query_ast = parser::parse_query(&opt.query)?;
    exctr.execute(query_ast)?;

    Ok(())
}
