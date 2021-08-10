use regex;

use super::parser::Parser;
use crate::error::Result;
use crate::model::RecordValue;

pub struct RegexParser {
    re: regex::bytes::Regex,
}

impl RegexParser {
    pub fn new(re_pattern: &str) -> Result<Self> {
        let re = regex::bytes::Regex::new(re_pattern).map_err(|e| ("bad regex pattern", e))?;

        Ok(Self { re })
    }
}

impl Parser for RegexParser {
    fn parse(&self, line: &[u8]) -> Result<Option<RecordValue>> {
        // TODO: handle named captures

        let caps = self.re.captures(line).ok_or("no match found")?;

        Ok(RecordValue::Array(
            caps.iter()
                .skip((self.re.captures_len() > 1) as usize)
                .map(|c| {
                    String::from_utf8(c.unwrap().as_bytes().to_owned())
                        .expect("only UTF-8 is supported")
                })
                .collect(),
        ))
    }
}
