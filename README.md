# pq - Parse and Query files with PromQL-like query language

Project is actively being developed!

New format:

```bash
tail -n 100 ~/access.log | ./target/debug/pq '/([^\s]+).+\[([^\s]+).*\]\s+"(\w+).*?"\s(\d+)\s+(\d+)/ | map {.0 as ip, .1:ts "%d/%b/%Y:%H:%M:%S", .2 as method, .3:str as status_code, .4:num as content_len} | select content_len[1s]'
```


## Why

I often find myself staring at Nginx or Envoy access logs `tail`ed to my screen
in real time.  My only wish at that moment is to be able to aggregate the lines
somehow and analyze the output at a slower pace. Ideally, with a familiar and 
concise (rules out SQL) query language:

Something like:

```bash
tail -f access.log | pq 'rate(requests{method="GET", status_code=~"5"}[1s])'
```


##  How

The idea is pretty straightforward:

**Parse an input stream into structured time series data
and then filter/transform/aggregate it with PromQL-like expression.**

For that, we need to consume the input record by record, 
parse each record into fields (e.g. using regex groups or pre-defined format parser),
and sort out fields into labels, metrics, and a timestamp. The resulting
stream of samples can be queried with PromQL-like language. And that's what `pq`
does - it implements the decoder, the query parser and executor, and the
encoder, to output the query results.


## Demo

Try it out yourself!

```bash
# Launch a test web server.
docker run -p 55055:80 --rm --name test_server nginx 2>/dev/null

# In another terminal, start pouring some well-known but diverse traffic.
# Notice, `-q` means Query Rate and `-c` means multiplier.
hey -n 1000000 -q 80 -c 2 -m GET http://localhost:55055/ &
hey -n 1000000 -q 60 -c 2 -m GET http://localhost:55055/qux &
hey -n 1000000 -q 40 -c 2 -m POST http://localhost:55055/ &
hey -n 1000000 -q 20 -c 2 -m PUT http://localhost:55055/foob &
hey -n 1000000 -q 10 -c 2 -m PATCH http://localhost:55055/ &
```

Access log in the first terminal looks impossible to analyze in real-time, right? `pq` to the rescue!

### Secondly HTTP request rate with by (method, status_code) breakdowns

```bash
docker logs -n 1000 -f test_server 2>/dev/null | \
    pq -d '[^\[]+\[([^\s]+).+?]\s+"([^\s]+)[^"]*?"\s+(\d+)\s+(\d+).*' \
        -e h \
        -t '0:%d/%b/%Y:%H:%M:%S' \
        -l 1:method \
        -l 2:status_code \
        -m 3:content_len \
        -- \
        'count_over_time(__line__[1s])'
```

![RPS](images/rps-2000-opt.png)


## Secondly traffic (in KB/s) aggregated by method

```bash
docker logs -n 1000 -f test_server 2>/dev/null | \
    pq -d '[^\[]+\[([^\s]+).+?]\s+"([^\s]+)[^"]*?"\s+(\d+)\s+(\d+).*' \
        -e h \
        -t '0:%d/%b/%Y:%H:%M:%S' \
        -l 1:method \
        -l 2:status_code \
        -m 3:content_len \
        -- \
        'sum(sum_over_time(content_len[1s])) by (method) / 1024'
```

![BPS](images/bps-2000-opt.png)

For more use cases, see [tests/scenarios folder](tests/scenarios).


## Usage

### Parse input

The input is seen as a stream of records. A typical record example is a single log line:

```
172.17.0.1 - - [24/Jun/2021:07:54:16 +0000] "GET / HTTP/1.1" 200 612 "-" "hey/0.0.1" "-"
```

A valid record must have a _timestamp_ and at least one numerical measurement, called _a metric_.
In the above example a metric is `body_bytes_sent` with the value of `612`. The timestamp
is obviously `24/Jun/2021:07:54:16 +0000`. A record can also have zero or more attributes
called _labels_. Typical label examples are: `ip`, `method`, `url`, `status_code`.

To parse a record, currently one can use a regular expression with groups. For instance,
the above log line can be parsed into 4 values using the following (scary looking) regex:

```bash
echo '172.17.0.1 - - [24/Jun/2021:07:54:16 +0000] "GET / HTTP/1.1" 200 612 "-" "hey/0.0.1" "-"' \
    pq -d '[^\[]+\[([^\s]+).+?]\s+"([^\s]+)[^"]*?"\s+(\d+)\s+(\d+).*' \
    ...
```

This regex will extract the timestamp, the request method and status code, and the response body size.

The next step is to name and type the extracted values:

```bash
echo '172.17.0.1 - - [24/Jun/2021:07:54:16 +0000] "GET / HTTP/1.1" 200 612 "-" "hey/0.0.1" "-"' \
    pq -d '[^\[]+\[([^\s]+).+?]\s+"([^\s]+)[^"]*?"\s+(\d+)\s+(\d+).*' \
        -t '0:%d/%b/%Y:%H:%M:%S' \
        -l 1:method \
        -l 2:status_code \
        -m 3:body_bytes \
        ...
```

where `-t` means _timestamp_, `-l` means _label_, and `-m` means _metric_, and then number before `:` 
represent the match position in the regex.

As a result, a time series called `body_bytes` is produced. Since a record in general can have
multiple metrics, parsing stage turns a single input stream into one or more time series called 
after the corresponding metric name.


### Query metrics

The query language is heavily influenced by PromQL. Hopefully existing PromQL
skills should be totally transferable.

Normally, a query starts from a metric selector:

- `body_bytes` - matches all records with the `body_bytes` metric.
- `body_bytes{method="GET"}` - takes only GET requests.
- `body_bytes{method!="GET", status_code~="5.."}` - takes failed non-GET requests.

A query is executed with a given frequency (by default _1 sec_) and a selector 
returns the latest closest sample from the stream. To get multiple samples, a time 
duration can be added:

- `body_bytes[1s]` - returns secondly buckets of samples
- `body_bytes{status_code!="200"}[1h30m15s5ms]` - returns all non-200 records for the past `~1h30m`.

An operator or a function can be applied to a selector.

Supported operators:

- arithmetic `+ - / * ^ %`: `body_bytes{method="GET"} + body_bytes{method="POST"}` or `body_bytes{} / 1024`
- comparison: `== != <= < >= >`: `body_bytes{} > 1000`
- aggregation `sum() min() max()`: `min(body_bytes)`
- coming soon - more aggregations `avg() topk() bottomk() count() quantile() ...`
- coming soon - logical `and unless or`

Supported functions:

- `avg_over_time(selector[duration])`
- `count_over_time(selector[duration])`
- `last_over_time(selector[duration])`
- `min_over_time(selector[duration])`
- `max_over_time(selector[duration])`
- `sum_over_time(selector[duration])`
- coming soon - other well-known functions...

And most of the expressions can be combined. Ex:

```SQL
sum(sum_over_time(content_len[1s])) by (method) / 1024
```

### Format output

Currently, only two formats are supported: 

- JSON - Prometheus API alike (default) 
- Human-readable (via `-e h`).


## Development

```bash
# Build it with
make

# Test it with
make test-all
make test-e2e

# Run a certain e2e test
E2E_CASE=vector_matching_one_to_one_010 make test-e2e
```

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

