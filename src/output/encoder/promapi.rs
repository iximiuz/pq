use serde::Serialize;
use serde_json;
use std::collections::HashMap;

use super::encoder::Encoder;
use crate::engine::{InstantVector, RangeVector, Value};
use crate::error::Result;

// Instant query - instant vector
// {
//   "resultType": "vector",
//   "result": [
//     {
//       "metric":{"foo":"123", "bar": "qux"},
//       "value": [1622104500, "10"]
//     },
//     {
//       "metric":{"foo":"456", "bar": "qux"},
//       "value": [1622104500, "20"]
//     }
//   ]
// }
#[derive(Serialize)]
struct VectorItem {
    metric: HashMap<String, String>,
    value: (f64, String),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Vector {
    result_type: &'static str,
    result: Vec<VectorItem>,
}

impl Vector {
    fn new(vector: &InstantVector) -> Self {
        Self {
            result_type: "vector",
            result: vector
                .samples()
                .iter()
                .map(|(labels, value)| VectorItem {
                    metric: labels.clone(),
                    value: (vector.timestamp() as f64 / 1000.0, value.to_string()),
                })
                .collect(),
        }
    }
}

// Instant query - range vector
// {
//   "resultType": "matrix",
//   "result": [
//     {
//       "metric": {"foo": "123", "bar": "qux"},
//       "values": [[1622104474.588,"0.938"], [1622104489.591,"0.94"]]
//     },
//     {
//       "metric": {"foo": "456", "bar": "qux"},
//       "values": [[1622104474.588,"0.938"], [1622104489.591,"0.97"]]
//     }
//   ]
// }
// #[derive(Serialize, Deserialize)]
// struct Matrix {}

// TODO: Instant query - scalar
// {
//   "resultType": "scalar",
//   "result": [1622104500, "42"]
// }

// TODO: Instant query - string
// {
//   "resultType": "string",
//   "result": [1622104500, "foo"]
// }

// TODO: Range query - scalar
// {
//   "resultType": "matrix",
//   "result": [        <--- always just one element
//     {
//       "metric": {},  <--- no metrics for a scalar
//       "values": [[1622103600, "42"], [1622103960, "42"]]
//     }
//   ]
// }

// TODO: Range query - instant vector
// {
//   "resultType": "matrix",
//   "result": [
//     {
//       "metric": {"foo": "123", "qux": "bar"},
//       "values": [[1622103600, "10"], [1622103960, "20"]]
//     },
//     {
//       "metric": {"foo": "456", "qux": "bar"},
//       "values": [[1622103600, "15"], [1622103960, "30"]]
//     }
//   ]
// }

// Range query - string => is not supported.
// Range query - range vector => is not supported.

pub struct PromApiEncoder {}

impl PromApiEncoder {
    pub fn new() -> Self {
        Self {}
    }

    // {
    //   "resultType": "vector",
    //   "result": [
    //     {
    //       "metric":{"foo":"123", "bar": "qux"},
    //       "value": [1622104500, "10"]
    //     },
    //     {
    //       "metric":{"foo":"456", "bar": "qux"},
    //       "value": [1622104500, "20"]
    //     }
    //   ]
    // }
    fn encode_instant_vector(&self, vector: &InstantVector) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&Vector::new(vector))
            .map_err(|e| ("JSON serialization failed", e))?)
    }

    fn encode_range_vector(&self, vector: &RangeVector) -> Result<Vec<u8>> {
        Ok(format!("OUTPUT VALUE {:?}", vector).bytes().collect())
    }
}

impl Encoder for PromApiEncoder {
    fn encode(&self, value: &Value) -> Result<Vec<u8>> {
        match value {
            Value::InstantVector(v) => self.encode_instant_vector(v),
            Value::RangeVector(v) => self.encode_range_vector(v),
            _ => unimplemented!(),
        }
    }
}
