use std::time::Duration;

use chrono::prelude::*;
use structopt::StructOpt;

use crate::error::Result;
use crate::model::Timestamp;
use crate::utils::parse::parse_duration;

#[derive(Debug, StructOpt)]
#[structopt(name = "pq", about = "pq command line arguments")]
pub struct CliOpt {
    // #[structopt(long = "v", short = "verbose")]
    // pub verbose: bool,

    // #[structopt(long = "encode", short = "e")]
    // pub encode: Option<String>,
    pub program: String,

    #[structopt(long = "since", short = "s", parse(try_from_str = parse_iso_time))]
    pub since: Option<Timestamp>,

    #[structopt(long = "until", short = "u", parse(try_from_str = parse_iso_time))]
    pub until: Option<Timestamp>,

    #[structopt(long = "interval", short = "i", parse(try_from_str = parse_duration))]
    pub interval: Option<Duration>,

    #[structopt(long = "lookback", short = "b", parse(try_from_str = parse_duration))]
    pub lookback: Option<Duration>,
}

fn parse_iso_time(s: &str) -> Result<Timestamp> {
    s.parse::<DateTime<Utc>>()
        .and_then(|t| Ok(t.timestamp_millis()))
        .map_err(|e| ("timestamp parsing failed", e).into())
}
