use std::collections::HashMap;

use crate::error::Result;

#[derive(Debug)]
pub enum Decoded {
    Tuple(usize, Vec<String>),
    Dict(usize, HashMap<String, String>),
}

pub trait Decoder {
    fn decode(line: &Vec<u8>) -> Result<Decoded>;
}
