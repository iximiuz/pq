use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::Path;
use std::rc::Rc;

use serde_json;
use structopt::StructOpt;

use pq::cliopt::CliOpt;
use pq::input::LineReader;
use pq::output::{LineWriter, Writer};
use pq::runner::Runner;
use pq::utils::time::TimeRange;

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
            &CliOpt::from_iter(cli_args.clone()),
        )?;

        let expected_output = expected(Box::new(io::BufReader::new(fs::File::open(
            test_dir.join("output"),
        )?)))?;

        assert_eq!(
            expected_output,
            actual_output,
            "\nUnexpected query result in '{}'.\nCommand: {}\nExpected:\n{}\nActual:\n{}",
            test_dir.display(),
            pprint_cli_args(&test_dir.join("input").as_path(), &cli_args),
            String::from_utf8_lossy(&expected_output),
            String::from_utf8_lossy(&actual_output),
        );
    }

    Ok(())
}

fn query<'a>(
    input_reader: Box<dyn io::BufRead>,
    cli_opt: &CliOpt,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let writer = Rc::new(RefCell::new(LineWriter::new(
        io::BufWriter::new(Vec::new()),
    )));

    struct MockWriter<W>(Rc<RefCell<W>>);

    impl<W: Writer> Writer for MockWriter<W> {
        fn write(&mut self, buf: &Vec<u8>) -> io::Result<()> {
            self.0.borrow_mut().write(buf)
        }
    }

    let mut runner = Runner::new(
        &cli_opt.program,
        Box::new(LineReader::new(input_reader)),
        Box::new(MockWriter(Rc::clone(&writer))),
        cli_opt.verbose,
        Some(TimeRange::new(cli_opt.since, cli_opt.until)?),
        cli_opt.interval,
        cli_opt.lookback,
    )?;
    runner.run()?;

    // To make Rc::try_unwrap(writer) happy.
    drop(runner);

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

fn pprint_cli_args(input_filename: &Path, command: &[String]) -> String {
    format!(
        "cat {} | {}",
        input_filename.display(),
        command
            .iter()
            .cloned()
            .map(|s| {
                if s == "pq" || s.starts_with("-") {
                    s
                } else {
                    format!("'{}'", s)
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    )
}
