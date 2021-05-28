use std::io::{self, BufReader};
use std::time::Duration;

use structopt::StructOpt;

use pq::engine::Executor;
use pq::input::{decoder::RegexDecoder, reader::LineReader, Input};
use pq::output::{encoder::PromApiEncoder, writer::LineWriter, Output};
use pq::parser;

#[derive(Debug, StructOpt)]
#[structopt(name = "pq", about = "pq command line arguments")]
struct CliOpt {
    #[structopt(long = "decode", short = "d")]
    decode: String,

    #[structopt(long = "timestamp", short = "t")]
    timestamp: String,

    #[structopt(long = "label", short = "l")]
    labels: Vec<String>,

    #[structopt(long = "metric", short = "m", required = true, min_values = 1)]
    metrics: Vec<String>,

    query: String,
}

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
        Box::new(PromApiEncoder::new()),
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
