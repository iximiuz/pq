use regex;

use super::decoder::{Decoded, Decoder};
use crate::error::Result;

pub struct RegexDecoder {
    re: regex::bytes::Regex,
}

impl RegexDecoder {
    pub fn new(re_pattern: &str) -> Result<Self> {
        let re = regex::bytes::Regex::new(re_pattern).map_err(|e| ("bad regex pattern", e))?;

        Ok(Self { re })
    }
}

impl Decoder for RegexDecoder {
    fn decode(&self, line: &Vec<u8>) -> Result<Decoded> {
        // TODO: handle named captures and return Decoded::Dict.

        let caps = self.re.captures(line).ok_or("no match found")?;

        Ok(Decoded::Tuple(
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
