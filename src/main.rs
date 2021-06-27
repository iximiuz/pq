use std::io::{self, BufReader};

use structopt::StructOpt;

use pq::cliopt::CliOpt;
use pq::common::time::TimeRange;
use pq::decoder::RegexDecoder;
use pq::encoder::HumanReadableEncoder;
use pq::input::LineReader;
use pq::output::LineWriter;
use pq::pipeline::Pipeline;
// use pq::query::{parse_query, Executor};

// -p '...'                             <--- just prints matching lines
// -p '...' -m '...'                    <--- prints matches (groups) or complaints if pattern doesn't match the regex
// -p '...' -m '...' -q '...'           <--- runs a PromQL query
// -p '...' -m '...' -q '...' -f '...'  <--- formats the output (JSON, PromQL, rust formatting, etc)
//
// -p '/\d+\s\w.../'
// -p '/\d+\s\w.../m'  <--- multiline, use -s '/<regex>/' to specify line separator
// -p '/(\d+)\s(\w).../' -m '[timestamp:%S, method:l, *, status_code:l, content_len:m]'
// -p '/(\d+)\s(\w).../' -m '[timestamp:%S, method:l, _, _, status_code:l, content_len:m]'
//
// -p json
// -p json -m '[...]'
// -p json -m '{timestamp:%S, method:l, content_len:m as bytes, *}'
// -p json -m '{timestamp:%S, method:l, content_len:m as bytes, _, _}'
//
// -p scanf
//
// # presets - parser and pattern matching, but the pattern matching part can be overriden
// -p apache
// -p envoy
// -p nginx
// -p nginx:combined
// -p redis

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = CliOpt::from_args();

    let mut pipeline = Pipeline::new(
        Box::new(LineReader::new(BufReader::new(io::stdin()))),
        Box::new(RegexDecoder::new(&opt.decode)?),
        Box::new(HumanReadableEncoder::new()),
        Box::new(LineWriter::new(io::stdout())),
    );

    pipeline.run()?;

    // let exctr = Executor::new(
    //     input,
    //     output,
    //     Some(TimeRange::new(opt.since, opt.until)?),
    //     opt.interval,
    //     opt.lookback,
    // );

    // let query_ast = parse_query(&opt.query)?;
    // exctr.execute(query_ast)?;

    Ok(())
}
