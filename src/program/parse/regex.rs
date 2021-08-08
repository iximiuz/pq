use regex;

use super::strategy::{DecodingResult, DecodingStrategy};
use crate::error::Result;

pub struct RegexDecodingStrategy {
    re: regex::bytes::Regex,
}

impl RegexDecodingStrategy {
    pub fn new(re_pattern: &str) -> Result<Self> {
        let re = regex::bytes::Regex::new(re_pattern).map_err(|e| ("bad regex pattern", e))?;

        Ok(Self { re })
    }
}

impl DecodingStrategy for RegexDecodingStrategy {
    fn decode(&self, line: &[u8]) -> Result<DecodingResult> {
        // TODO: handle named captures and return Decoded::Dict.

        let caps = self.re.captures(line).ok_or("no match found")?;

        Ok(DecodingResult::Tuple(
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
