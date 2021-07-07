use std::collections::HashMap;

use crate::error::Result;

#[derive(Debug)]
pub enum DecodingResult {
    Tuple(Vec<String>),
    Dict(HashMap<String, String>),
}

pub trait DecodingStrategy {
    fn decode(&self, line: &Vec<u8>) -> Result<DecodingResult>;
}
