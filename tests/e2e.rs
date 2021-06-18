use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use serde_json;
use structopt::StructOpt;

use pq::cliopt::CliOpt;
use pq::engine::Executor;
use pq::input::{decoder::RegexDecoder, reader::LineReader, Input};
use pq::output::{
    encoder::PromApiEncoder,
    writer::{LineWriter, Writer},
    Output,
};
use pq::parser;

#[test]
fn e2e() -> Result<(), Box<dyn std::error::Error>> {
    let root_test_dir = Path::new(file!()).parent().unwrap().join("scenarios");

    for test_dir in fs::read_dir(&root_test_dir)? {
        let test_dir = test_dir?.path();

        if let Ok(filter) = std::env::var("E2E_CASE") {
            if !test_dir.as_os_str().to_string_lossy().ends_with(&filter) {
                continue;
            }
        }

        let cli_args: Vec<String> =
            serde_json::from_str(&fs::read_to_string(test_dir.join("args.json"))?)?;

        let actual_output = query(
            Box::new(io::BufReader::new(fs::File::open(test_dir.join("input"))?)),
            &cli_args,
        )?;

        let expected_output = expected(Box::new(io::BufReader::new(fs::File::open(
            test_dir.join("output"),
        )?)))?;

        assert_eq!(
            expected_output,
            actual_output,
            "\nUnexpected query result in '{}'.\nExpected:\n{}\nActual:\n{}",
            test_dir.display(),
            String::from_utf8_lossy(&expected_output),
            String::from_utf8_lossy(&actual_output),
        );
    }

    Ok(())
}

fn query<'a>(
    input_reader: Box<dyn io::BufRead>,
    cli_args: &[String],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let opt = CliOpt::from_iter(cli_args);

    let input = Input::new(
        Box::new(LineReader::new(input_reader)),
        Box::new(RegexDecoder::new(
            &opt.decode,
            opt.timestamp,
            opt.labels,
            opt.metrics,
        )?),
    );

    let writer = Rc::new(RefCell::new(LineWriter::new(
        io::BufWriter::new(Vec::new()),
    )));

    struct TestWriter<W>(Rc<RefCell<W>>);

    impl<W: Writer> Writer for TestWriter<W> {
        fn write(&mut self, buf: &Vec<u8>) -> io::Result<()> {
            self.0.borrow_mut().write(buf)
        }
    }

    let output = Output::new(
        Box::new(TestWriter(Rc::clone(&writer))),
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

    // To make Rc::try_unwrap(writer) work.
    drop(exctr);

    let writer = match Rc::try_unwrap(writer) {
        Ok(writer) => writer,
        _ => unreachable!(),
    };

    Ok(writer.into_inner().into_inner().into_inner()?)
}

fn expected(mut reader: Box<dyn io::BufRead>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    Ok(buf)
}
