use std::collections::HashMap;

use crate::error::Result;

#[derive(Debug)]
pub enum Entry {
    List(Vec<String>),
    Dict(HashMap<String, String>),
}

pub trait Decoder {
    fn decode(&self, buf: &Vec<u8>) -> Result<Entry>;
}
