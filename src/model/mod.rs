mod labels;
mod record;
mod timestamp;

pub use labels::*;
pub use record::*;
pub use timestamp::*;

pub type MetricName = String;

pub type SampleValue = f64;
