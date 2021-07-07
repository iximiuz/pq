use std::time::Duration;

use structopt::StructOpt;

use crate::error::{Error, Result};
use crate::model::Timestamp;
use crate::utils::{parse::parse_duration, time::try_parse_time};

#[derive(Debug, StructOpt)]
#[structopt(name = "pq", about = "pq command line arguments")]
pub struct CliOpt {
    pub program: String,

    #[structopt(long = "since", short = "s", parse(try_from_str = parse_time))]
    pub since: Option<Timestamp>,

    #[structopt(long = "until", short = "u", parse(try_from_str = parse_time))]
    pub until: Option<Timestamp>,

    #[structopt(long = "interval", short = "I", parse(try_from_str = parse_duration))]
    pub interval: Option<Duration>,

    #[structopt(long = "lookback", short = "b", parse(try_from_str = parse_duration))]
    pub lookback: Option<Duration>,

    #[structopt(long = "i", short = "interactive")]
    pub interactive: bool,

    #[structopt(long = "v", short = "verbose")]
    pub verbose: bool,
}

fn parse_time(s: &str) -> Result<Timestamp> {
    match try_parse_time(s) {
        Some(t) => Ok(t),
        None => Err(Error::new("couldn't guess time format")),
    }
}
