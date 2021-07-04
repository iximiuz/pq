use std::collections::{HashMap, HashSet};
use std::fmt;

use chrono::prelude::*;
use lazy_static::lazy_static;

use super::decoder::{Decoder, Entry};
use crate::error::{Error, Result};
use crate::program::Mapper as MappingRules;

// [timestamp:<format>, method:l, status_code:l, body_size:m]

pub struct MappingStrategy {
    timestamp_cap: CaptureTimestamp,
    label_caps: Vec<CaptureLabel>,
    metric_caps: Vec<CaptureMetric>,
}

impl MappingStrategy {
    pub fn new(
        re_pattern: &str,
        timestamp: String,
        labels: Vec<String>,
        metrics: Vec<String>,
    ) -> Result<Self> {
        let re = regex::bytes::Regex::new(re_pattern).map_err(|e| ("bad regex pattern", e))?;

        let timestamp_cap = CaptureTimestamp::parse(timestamp)?;
        let label_caps = labels
            .into_iter()
            .map(CaptureLabel::parse)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let metric_caps = metrics
            .into_iter()
            .map(CaptureMetric::parse)
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Self::validate_captures(&re, &timestamp_cap, &label_caps, &metric_caps)?;

        Ok(Self {
            re,
            timestamp_cap,
            label_caps,
            metric_caps,
        })
    }

    fn validate_captures(
        re: &regex::bytes::Regex,
        timestamp: &CaptureTimestamp,
        labels: &[CaptureLabel],
        metrics: &[CaptureMetric],
    ) -> Result<()> {
        if re.captures_len() < 1 {
            return Err(Error::new(
                "regex must have at least two captures (timestamp and metric)",
            ));
        }
        if re.captures_len() - 2 < labels.len() + metrics.len() {
            return Err(Error::new(
                "too few regex captures or too many metrics/labels",
            ));
        }
        if re.captures_len() - 2 > labels.len() + metrics.len() {
            return Err(Error::new(
                "too many regex captures or too few metrics/labels",
            ));
        }

        let mut unique_positions = HashSet::new();
        unique_positions.insert(timestamp.pos);

        let mut unique_names = HashSet::new();

        let max_capture = re.captures_len();

        for (pos, name) in labels
            .iter()
            .map(|cap| (cap.pos, &cap.name))
            .chain(metrics.iter().map(|cap| (cap.pos, &cap.name)))
        {
            if pos > max_capture {
                return Err(Error::from(format!(
                    "out of bound capture position {}; max allowed position is {}",
                    pos, max_capture
                )));
            }
            if !unique_positions.insert(pos) {
                return Err(Error::from(format!("ambiguous capture position {}", pos)));
            }
            if !unique_names.insert(name) {
                return Err(Error::from(format!("ambiguous capture name {}", name)));
            }
        }

        Ok(())
    }
}

impl Decoder for RegexDecoder {
    fn decode(&self, buf: &Vec<u8>) -> Result<Entry> {
        let record_caps = self.re.captures(buf).ok_or("no match found")?;

        let timestamp = parse_record_timestamp(
            &String::from_utf8(
                record_caps
                    .get(self.timestamp_cap.pos + 1)
                    .ok_or("timestamp capture is empty")?
                    .as_bytes()
                    .to_vec(),
            )
            .map_err(|e| ("couldn't decode UTF-8 timestamp value", e))?,
            Some(&self.timestamp_cap.format),
        )?;

        let mut metrics = Values::new();
        for metric_cap in self.metric_caps.iter() {
            if let Some(metric) = record_caps.get(metric_cap.pos + 1) {
                metrics.insert(
                    metric_cap.name.clone(),
                    String::from_utf8(metric.as_bytes().to_vec())
                        .map_err(|e| ("couldn't decode UTF-8 metric value", e))?
                        .parse::<f64>()
                        .map_err(|e| ("couldn't parse metric value into f64", e))?,
                );
            }
        }

        if metrics.len() == 0 {
            return Err(Error::new("no metric match found"));
        }

        let mut labels = HashMap::new();
        for label_cap in self.label_caps.iter() {
            if let Some(label) = record_caps.get(label_cap.pos + 1) {
                labels.insert(
                    label_cap.name.clone(),
                    String::from_utf8(label.as_bytes().to_vec())
                        .map_err(|e| ("couldn't decode UTF-8 label value", e))?,
                );
            }
        }

        Ok(Record(timestamp.timestamp_millis(), labels, metrics))
    }
}

struct CaptureTimestamp {
    pos: usize,
    format: String,
}

impl CaptureTimestamp {
    fn parse(capstr: String) -> Result<Self> {
        lazy_static! {
            static ref RE: regex::Regex = regex::Regex::new(r"^([0-9]{1,3}):(.{1,256})$").unwrap();
        }

        let caps = RE
            .captures(&capstr)
            .ok_or("malformed timestamp capture string")?;

        let pos = caps[1]
            .parse::<usize>()
            .map_err(|e| ("unsupported timestamp capture position", e))?;

        let format = caps[2].into();

        Ok(Self { pos, format })
    }
}

struct CaptureLabel {
    pos: usize,
    name: String,
}

impl CaptureLabel {
    fn parse(capstr: String) -> Result<Self> {
        let (pos, name) = parse_named_capture_str(&capstr, NamedCaptureKind::Label)?;
        Ok(Self { pos, name })
    }
}

struct CaptureMetric {
    pos: usize,
    name: String,
}

impl CaptureMetric {
    fn parse(capstr: String) -> Result<Self> {
        let (pos, name) = parse_named_capture_str(&capstr, NamedCaptureKind::Metric)?;
        Ok(Self { pos, name })
    }
}

#[derive(Debug)]
enum NamedCaptureKind {
    Label,
    Metric,
}

impl fmt::Display for NamedCaptureKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

fn parse_named_capture_str(capstr: &str, kind: NamedCaptureKind) -> Result<(usize, String)> {
    lazy_static! {
        static ref RE: regex::Regex =
            regex::Regex::new(r"^([0-9]{1,3})(:[a-zA-Z_][a-zA-Z0-9_]{0,256})?$").unwrap();
    }

    let caps = RE
        .captures(capstr)
        .ok_or(format!("malformed {} capture string", kind))?;

    let pos = caps[1]
        .parse::<usize>()
        .map_err(|e| (format!("unsupported {} capture position", kind), e))?;

    match caps.get(2) {
        None => Ok((pos, format!("{}{}", kind, pos))),
        Some(name) => Ok((pos, name.as_str()[1..].into())),
    }
}

fn parse_record_timestamp(timestamp: &str, format: Option<&str>) -> Result<DateTime<Utc>> {
    match format {
        Some(f) => Utc.datetime_from_str(timestamp, f),
        None => timestamp.parse::<DateTime<Utc>>(),
    }
    .map_err(|e| (Error::from(("couldn't parse timestamp of a record", e))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_decoder_new() -> std::result::Result<(), String> {
        RegexDecoder::new(
            r"(\d+)\s(\w)\s(\d+)".into(),
            "0:YYYY".into(),
            vec!["1:firstname".into()],
            vec!["2:age".into()],
        )?;
        Ok(())
    }

    #[test]
    fn test_regex_decoder_new_error() -> std::result::Result<(), String> {
        match RegexDecoder::new("".into(), "".into(), vec![], vec!["".into()]) {
            Err(e) => {
                assert_eq!(e.message(), "malformed timestamp capture string");
            }
            Ok(_) => {
                return Err(
                    "call should have failed with error but returned a decoder instead".into(),
                )
            }
        };

        match RegexDecoder::new(
            r"(\d+)\s(\w)\s(\d+)".into(),
            "0:YYYY".into(),
            vec![],
            vec!["2:age".into()],
        ) {
            Err(e) => {
                assert_eq!(
                    e.message(),
                    "too many regex captures or too few metrics/labels"
                );
            }
            Ok(_) => {
                return Err(
                    "call should have failed with error but returned a decoder instead".into(),
                )
            }
        };

        match RegexDecoder::new(
            r"(\d+)\s(\w)\s(\d+)".into(),
            "0:YYYY".into(),
            vec!["1:firstname".into(), "2:lastname".into()],
            vec!["3:age".into()],
        ) {
            Err(e) => {
                assert_eq!(
                    e.message(),
                    "too few regex captures or too many metrics/labels"
                );
            }
            Ok(_) => {
                return Err(
                    "call should have failed with error but returned a decoder instead".into(),
                )
            }
        };

        Ok(())
    }

    #[test]
    fn test_capture_timestamp_parse() -> std::result::Result<(), String> {
        let cap = CaptureTimestamp::parse("0:YYYY-MM-DD".into())?;
        assert_eq!(cap.pos, 0);
        assert_eq!(cap.format, "YYYY-MM-DD");
        Ok(())
    }

    #[test]
    fn test_parse_named_capture_str_wellformed() -> std::result::Result<(), String> {
        let (pos, name) = parse_named_capture_str("0", NamedCaptureKind::Label)?;
        assert_eq!(pos, 0);
        assert_eq!(name, "label0");

        let (pos, name) = parse_named_capture_str("1", NamedCaptureKind::Metric)?;
        assert_eq!(pos, 1);
        assert_eq!(name, "metric1");

        let (pos, name) = parse_named_capture_str("100:foo", NamedCaptureKind::Metric)?;
        assert_eq!(pos, 100);
        assert_eq!(name, "foo");

        let (pos, name) = parse_named_capture_str("999:f42", NamedCaptureKind::Metric)?;
        assert_eq!(pos, 999);
        assert_eq!(name, "f42");

        let (pos, name) = parse_named_capture_str("000:FOO", NamedCaptureKind::Metric)?;
        assert_eq!(pos, 0);
        assert_eq!(name, "FOO");

        let (pos, name) = parse_named_capture_str("2:_", NamedCaptureKind::Metric)?;
        assert_eq!(pos, 2);
        assert_eq!(name, "_");

        let (pos, name) = parse_named_capture_str("3:__", NamedCaptureKind::Metric)?;
        assert_eq!(pos, 3);
        assert_eq!(name, "__");

        let (pos, name) = parse_named_capture_str("4:_foo_", NamedCaptureKind::Metric)?;
        assert_eq!(pos, 4);
        assert_eq!(name, "_foo_");

        Ok(())
    }

    #[test]
    fn test_parse_named_capture_str_malformed() -> std::result::Result<(), String> {
        match parse_named_capture_str("", NamedCaptureKind::Label) {
            Err(e) => {
                assert_eq!(e.message(), "malformed label capture string");
            }
            Ok(v) => {
                return Err(format!(
                    "call should have failed with error but returned {:?} instead",
                    v
                ))
            }
        };

        match parse_named_capture_str("x", NamedCaptureKind::Label) {
            Err(e) => {
                assert_eq!(e.message(), "malformed label capture string");
            }
            Ok(v) => {
                return Err(format!(
                    "call should have failed with error but returned {:?} instead",
                    v
                ))
            }
        };

        match parse_named_capture_str("-1", NamedCaptureKind::Label) {
            Err(e) => {
                assert_eq!(e.message(), "malformed label capture string");
            }
            Ok(v) => {
                return Err(format!(
                    "call should have failed with error but returned {:?} instead",
                    v
                ))
            }
        };

        match parse_named_capture_str("1:", NamedCaptureKind::Label) {
            Err(e) => {
                assert_eq!(e.message(), "malformed label capture string");
            }
            Ok(v) => {
                return Err(format!(
                    "call should have failed with error but returned {:?} instead",
                    v
                ))
            }
        };

        match parse_named_capture_str("a:1", NamedCaptureKind::Label) {
            Err(e) => {
                assert_eq!(e.message(), "malformed label capture string");
            }
            Ok(v) => {
                return Err(format!(
                    "call should have failed with error but returned {:?} instead",
                    v
                ))
            }
        };

        match parse_named_capture_str("1:1", NamedCaptureKind::Label) {
            Err(e) => {
                assert_eq!(e.message(), "malformed label capture string");
            }
            Ok(v) => {
                return Err(format!(
                    "call should have failed with error but returned {:?} instead",
                    v
                ))
            }
        };

        match parse_named_capture_str("1:1foo", NamedCaptureKind::Label) {
            Err(e) => {
                assert_eq!(e.message(), "malformed label capture string");
            }
            Ok(v) => {
                return Err(format!(
                    "call should have failed with error but returned {:?} instead",
                    v
                ))
            }
        };

        match parse_named_capture_str("1000:foo", NamedCaptureKind::Label) {
            Err(e) => {
                assert_eq!(e.message(), "malformed label capture string");
            }
            Ok(v) => {
                return Err(format!(
                    "call should have failed with error but returned {:?} instead",
                    v
                ))
            }
        };

        Ok(())
    }
}
