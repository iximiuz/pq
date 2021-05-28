use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "pq", about = "pq command line arguments")]
pub struct CliOpt {
    #[structopt(long = "decode", short = "d")]
    pub decode: String,

    #[structopt(long = "timestamp", short = "t")]
    pub timestamp: String,

    #[structopt(long = "label", short = "l")]
    pub labels: Vec<String>,

    #[structopt(long = "metric", short = "m", required = true, min_values = 1)]
    pub metrics: Vec<String>,

    pub query: String,
}
