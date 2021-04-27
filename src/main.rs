use std::io::{self, BufReader};

use structopt::StructOpt;

use pq::decoder::RegexDecoder;
use pq::input::Input;
use pq::parser;
use pq::reader::LineReader;

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

    let mut input = Input::new(
        Box::new(LineReader::new(BufReader::new(io::stdin()))),
        Box::new(RegexDecoder::new(
            &opt.decode,
            opt.timestamp,
            opt.labels,
            opt.metrics,
        )?),
    );

    let ast = parser::parse(&opt.query)?;
    println!("AST={:?}", ast);

    loop {
        let record = match input.take_one()? {
            Some(r) => r,
            None => break,
        };
        println!("{:?}", record);
    }

    Ok(())
}
