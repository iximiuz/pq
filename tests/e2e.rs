use std::fs;
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;
use std::time::Duration;

use serde_json;
use structopt::StructOpt;

use pq::cliopt::CliOpt;
use pq::engine::Executor;
use pq::input::{decoder::RegexDecoder, reader::LineReader, Input};
use pq::output::{encoder::PromApiEncoder, writer::LineWriter, Output};
use pq::parser;

#[test]
fn e2e() -> Result<(), Box<dyn std::error::Error>> {
    let root_test_dir = Path::new(file!()).parent().unwrap().join("scenarios");

    for test_dir in fs::read_dir(&root_test_dir)? {
        let test_dir = test_dir?.path();

        let args: Vec<String> =
            serde_json::from_str(&fs::read_to_string(test_dir.join("args.json"))?)?;

        let actual_output = query(
            Box::new(BufReader::new(fs::File::open(test_dir.join("input"))?)),
            &args,
        )?;

        let expected_output = expected(Box::new(BufReader::new(fs::File::open(
            test_dir.join("output"),
        )?)))?;

        assert_eq!(expected_output, actual_output);
    }

    Ok(())
}

fn query<'a>(
    input_reader: Box<dyn BufRead>,
    args: &[String],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let opt = CliOpt::from_iter(args);

    let input = Input::new(
        Box::new(LineReader::new(input_reader)),
        Box::new(RegexDecoder::new(
            &opt.decode,
            opt.timestamp,
            opt.labels,
            opt.metrics,
        )?),
    );

    let output = Output::new(
        Box::new(LineWriter::new(BufWriter::new(Vec::new()))),
        Box::new(PromApiEncoder::new()),
    );

    let exctr = Executor::new(
        input,
        output,
        None,
        Some(Duration::from_millis(1000)),
        Some(Duration::from_millis(1000)),
    );

    let query_ast = parser::parse_query(&opt.query)?;
    exctr.execute(query_ast)?;

    Ok(exctr.output().into_inner().into_inner().into_inner()?)
}

fn expected(mut reader: Box<dyn BufRead>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    Ok(buf)
}
