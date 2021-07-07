use std::io::{self, BufReader};

use structopt::StructOpt;

use pq::cliopt::CliOpt;
use pq::input::LineReader;
use pq::output::LineWriter;
use pq::runner::Runner;
use pq::utils::time::TimeRange;

//    'json'                                 // as is
//    'json | map {.foo:f64 as bar}'         // take only records with 'foo' attr, result consists of a single-field object
//    'json | map {.foo:f64 as bar, *}'      // take only records with 'foo' attr, but also keep all other records in the resulting object
//    '/.*(\d+)\s(\w+)/ | map {.0:ts "%Y-%m-%d" as time, .1 as method, extra_label: "value"}'
//    'csv (name, city, age)'
//    'csv (name, city, age) | map {...}'
//    'promql'
//    'promql | map {*, foo:42}'
//    'influxdb'
//    'nginx  | to_json'
//    'apache | to_logfmt'
//    '...'
//
//
//    '/.*(\d+)\s(\w+)/
//    | map {
//      .0:ts with format "%Y-%m-%d" as time,
//      .1 as method,
//      extra_label: "value"
//    }
//    | select duration{method!="GET"}
//    | to_json'

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = CliOpt::from_args();

    let mut runner = Runner::new(
        &opt.program,
        Box::new(LineReader::new(BufReader::new(io::stdin()))),
        Box::new(LineWriter::new(io::stdout())),
        opt.verbose,
        opt.interactive,
        Some(TimeRange::new(opt.since, opt.until)?),
        opt.interval,
        opt.lookback,
    )?;

    runner.run()?;

    Ok(())
}
