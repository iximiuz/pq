use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::Path;
use std::rc::Rc;

use structopt::StructOpt;

use pq::cliopt::CliOpt;
use pq::input::LineReader;
use pq::output::{LineWriter, Writer};
use pq::runner::{Runner, RunnerOptions};
use pq::utils::time::TimeRange;

#[test]
fn e2e() -> Result<(), Box<dyn std::error::Error>> {
    let root_test_dir = Path::new(file!()).parent().unwrap().join("scenarios");
    let mut failed = Vec::new();

    for test_dir in fs::read_dir(&root_test_dir)? {
        let test_dir = test_dir?.path();

        if let Ok(filter) = std::env::var("E2E_CASE") {
            if !test_dir.as_os_str().to_string_lossy().ends_with(&filter) {
                continue;
            }
        }

        eprintln!("{}: running...", test_dir.display());

        let cli_args: Vec<String> =
            serde_json::from_str(&fs::read_to_string(test_dir.join("args.json"))?)?;

        let actual_output = match query(
            Box::new(io::BufReader::new(fs::File::open(test_dir.join("input"))?)),
            &cli_args,
        ) {
            Ok(actual_output) => actual_output,
            Err(e) => {
                eprintln!("{}: query failed with '{}'", test_dir.display(), e);
                failed.push(test_dir);
                continue;
            }
        };

        let expected_output = expected(Box::new(io::BufReader::new(fs::File::open(
            test_dir.join("output"),
        )?)))?;

        if expected_output == actual_output {
            eprintln!("{}: ok!", test_dir.display());
        } else {
            eprintln!(
                "{}: unexpected query result.\nCommand: {}\nExpected:\n{}\nActual:\n{}",
                test_dir.display(),
                pprint_cli_args(test_dir.join("input").as_path(), &cli_args),
                String::from_utf8_lossy(&expected_output),
                String::from_utf8_lossy(&actual_output),
            );
            failed.push(test_dir);
        }
    }

    assert!(
        failed.is_empty(),
        "Failed e2e tests:\n{}",
        failed
            .iter()
            .map(|p| format!("\t- {}", p.display()))
            .collect::<Vec<String>>()
            .join("\n")
    );
    Ok(())
}

fn query(
    input_reader: Box<dyn io::BufRead>,
    cli_args: &[String],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let writer = Rc::new(RefCell::new(LineWriter::new(
        io::BufWriter::new(Vec::new()),
    )));

    struct MockWriter<W>(Rc<RefCell<W>>);

    impl<W: Writer> Writer for MockWriter<W> {
        fn write(&mut self, buf: &[u8]) -> io::Result<()> {
            self.0.borrow_mut().write(buf)
        }
    }

    let cli_opt = CliOpt::from_iter_safe(cli_args)?;

    let mut runner = Runner::new(
        &cli_opt.program,
        Box::new(LineReader::new(input_reader)),
        Box::new(MockWriter(Rc::clone(&writer))),
        RunnerOptions::new(
            cli_opt.verbose,
            cli_opt.interactive,
            Some(TimeRange::new(cli_opt.since, cli_opt.until)?),
            cli_opt.interval,
            cli_opt.lookback,
        ),
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
                if s == "pq" || s.starts_with('-') {
                    s
                } else {
                    format!("'{}'", s)
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    )
}
