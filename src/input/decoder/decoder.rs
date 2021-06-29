use std::collections::HashMap;

use crate::error::Result;

#[derive(Debug)]
pub enum Decoded {
    Tuple(Vec<String>),
    Map(HashMap<String, String>),
}

pub trait Decoder {
    fn decode(&self, buf: &Vec<u8>) -> Result<Decoded>;
}
