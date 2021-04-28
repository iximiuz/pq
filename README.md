# pq - query textual streams with PromQL

## Glossary

- Time Series - a stream of timestamped values, _aka_ samples sharing the same metric name and, optionally, the same set of labels (i.e. a unique combination of key-value pairs).
- Metric name - a human-readable name of a measurement. E.g. `http_requests_total`, `content_length`, etc).
- Metric type - counter, gauge, histogram, and summary.
- Label - a dimension of the measurement. E.g. `method`, `url`, etc.
- Sample - _aka_ data point - a (value, timestamp) tuple. Value is always float64 and timestamp is always with millisecond precision.
- Instant vector - a type of expression evaluation - a set of time series (vector) containing a single sample for each time series, all sharing the same timestamp.
- Range vector - a type of expression evaluation - a set of time series containing a range of data points over time for each time series.
- Scalar and string - two other expression evaluation results.
- Vector selector - expression of a form `<metric_name>[{label1=value1[, label2=value2, ...]}][[time_duration]]`.

## Run

```bash
cargo test

cat | cargo run -- '{name="bob"}' -d '([^\s]+)\s(\w+)\s(\d+)' -t '0:%Y-%m-%dT%H:%M:%S' -l 1:name -m 2:age <<EOF
2021-01-01T05:40:41 bob 42
2021-01-01T06:30:10 sarah 25
2022-01-01T05:40:41 bob 42
2022-01-01T06:30:10 sarah 26
EOF
```

