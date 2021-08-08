use std::collections::HashMap;

use serde_json::{self, Map, Value};

use super::strategy::{DecodingResult, DecodingStrategy};
use crate::error::{Error, Result};

pub struct JSONDecodingStrategy {}

impl Default for JSONDecodingStrategy {
    fn default() -> Self {
        Self {}
    }
}

impl JSONDecodingStrategy {
    fn decode_tuple(&self, tuple: Vec<Value>) -> Result<DecodingResult> {
        let items: Vec<String> = tuple
            .iter()
            .filter_map(|v| match v {
                Value::Bool(b) => Some(b.to_string()),
                Value::Null => Some("null".to_string()),
                Value::Number(n) => Some(n.to_string()),
                Value::String(s) => Some(s.to_string()),
                _ => None,
            })
            .collect();

        Ok(DecodingResult::Tuple(items))
    }

    fn decode_dict(&self, dict: Map<String, Value>) -> Result<DecodingResult> {
        let items: HashMap<String, String> = dict
            .iter()
            .filter_map(|(k, v)| match v {
                Value::Bool(b) => Some((k.clone(), b.to_string())),
                Value::Null => Some((k.clone(), "null".to_string())),
                Value::Number(n) => Some((k.clone(), n.to_string())),
                Value::String(s) => Some((k.clone(), s.to_string())),
                _ => None,
            })
            .collect();

        Ok(DecodingResult::Dict(items))
    }
}

impl DecodingStrategy for JSONDecodingStrategy {
    fn decode(&self, line: &[u8]) -> Result<DecodingResult> {
        match serde_json::from_slice(line) {
            Ok(Value::Array(t)) => self.decode_tuple(t),
            Ok(Value::Object(o)) => self.decode_dict(o),
            Err(e) => Err(("JSON decoding failed", e).into()),
            _ => Err(Error::new(
                "JSON decoder supports only flat arrays and objects",
            )),
        }
    }
}
