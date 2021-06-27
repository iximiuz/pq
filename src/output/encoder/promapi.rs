use std::collections::BTreeMap;

use serde::Serialize;
use serde_json;

use super::encoder::Encoder;
use crate::error::Result;
use crate::query::{ExprValue, InstantVector, RangeVector};

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

#[derive(Serialize)]
struct VectorItem {
    metric: BTreeMap<String, String>,
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
                    metric: labels.clone().into_iter().collect(), // to make the label order deterministic
                    value: (vector.timestamp() as f64 / 1000.0, value.to_string()),
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct MatrixItem {
    metric: BTreeMap<String, String>,
    values: Vec<(f64, String)>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Matrix {
    result_type: &'static str,
    result: Vec<MatrixItem>,
}

impl Matrix {
    fn new(vector: &RangeVector) -> Self {
        Self {
            result_type: "matrix",
            result: vector
                .samples()
                .iter()
                .map(|(labels, values)| MatrixItem {
                    metric: labels.clone().into_iter().collect(), // to make the label order deterministic
                    values: values
                        .iter()
                        .rev()
                        .map(|(val, ts)| (*ts as f64 / 1000.0, val.to_string()))
                        .collect(),
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Scalar {
    result_type: &'static str,
    result: (f64, String),
}

impl Scalar {
    fn new(n: f64) -> Self {
        Self {
            result_type: "scalar",
            result: (0.0, n.to_string()),
        }
    }
}

pub struct PromApiEncoder {}

impl PromApiEncoder {
    pub fn new() -> Self {
        Self {}
    }

    // Instant query - instant vector
    // {
    //   "resultType": "vector",
    //   "result": [
    //     {
    //       "metric":{"bar":"123", "foo": "qux"},
    //       "value": [1622104500, "10"]
    //     },
    //     {
    //       "metric":{"bar":"456", "foo": "qux"},
    //       "value": [1622104500, "20"]
    //     }
    //   ]
    // }
    fn encode_instant_vector(&self, vector: &InstantVector) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&Vector::new(vector))
            .map_err(|e| ("JSON serialization failed", e))?)
    }

    // Instant query - range vector
    // {
    //   "resultType": "matrix",
    //   "result": [
    //     {
    //       "metric": {"bar": "123", "foo": "qux"},
    //       "values": [[1622104474.588,"0.938"], [1622104489.591,"0.94"]]
    //     },
    //     {
    //       "metric": {"bar": "456", "foo": "qux"},
    //       "values": [[1622104474.588,"0.938"], [1622104489.591,"0.97"]]
    //     }
    //   ]
    // }
    fn encode_range_vector(&self, vector: &RangeVector) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&Matrix::new(vector))
            .map_err(|e| ("JSON serialization failed", e))?)
    }

    // Instant query - scalar
    // {
    //   "resultType": "scalar",
    //   "result": [1622104500, "42"]
    // }
    fn encode_scalar(&self, number: f64) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&Scalar::new(number))
            .map_err(|e| ("JSON serialization failed", e))?)
    }
}

impl Encoder for PromApiEncoder {
    fn encode(&self, value: &ExprValue) -> Result<Vec<u8>> {
        match value {
            ExprValue::InstantVector(v) => self.encode_instant_vector(v),
            ExprValue::RangeVector(v) => self.encode_range_vector(v),
            ExprValue::Scalar(v) => self.encode_scalar(*v),
        }
    }
}
