use std::io::{self, BufReader};
use std::time::Duration;

use structopt::StructOpt;

use pq::cliopt::CliOpt;
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
        None,
        Some(Duration::from_millis(1000)),
        Some(Duration::from_millis(1000)),
    );

    let query_ast = parser::parse_query(&opt.query)?;
    exctr.execute(query_ast)?;

    Ok(())
}
