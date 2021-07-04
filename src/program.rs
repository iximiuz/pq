use crate::error::{Error, Result};
use crate::utils::parse::{maybe_lpadded, IResult, ParseError, Span};

use nom::{
    branch::alt,
    bytes::complete::{tag_no_case, take},
    character::complete::char,
    sequence::preceded,
};

#[derive(Debug)]
pub struct AST {
    pub decoder: Decoder,
    pub mapper: Option<Mapper>,
    pub query: Option<String>,
    pub formatter: Option<Formatter>,
}

#[derive(Debug)]
pub enum Decoder {
    JSON,
    Regex { regex: String },
    // TODO:
    // CSV {
    //     header: Vec<String>,
    //     separator: String,
    // },
    // Prometheus,
    // InfluxDB,
    // Nginx,
    // Apache,
    // Envoy,
}

#[derive(Debug)]
pub struct Mapper {}

#[derive(Debug)]
pub enum Formatter {
    // Prometheus,
    HumanReadable,
    JSON,
}

pub fn parse_program(program: &str) -> Result<AST> {
    match do_parse_program(Span::new(program.trim())) {
        Ok((rest, _)) if rest.len() > 0 => Err(Error::from(
            ParseError::partial("program", "EOF", rest).message(),
        )),
        Ok((_, ast)) => Ok(ast),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => Err(Error::from(e.message())),
        Err(nom::Err::Incomplete(_)) => unreachable!(),
    }
}

fn do_parse_program(input: Span) -> IResult<AST> {
    // Parse decoder - the only one mandatory part of the program.
    let (rest, decoder) = match decoder(input) {
        Ok((rest, decoder)) => (rest, decoder),
        Err(nom::Err::Error(_)) => return Err(nom::Err::Failure(ParseError::new(
            "a valid pq program must start from a known parser (supported parsers - regex, JSON)"
                .to_owned(),
            input,
        ))),
        Err(e) => return Err(e),
    };

    let (rest, mapper) = match maybe_lpadded(preceded(char('|'), maybe_lpadded(mapper)))(rest) {
        Ok((rest, mapper)) => (rest, Some(mapper)),
        Err(nom::Err::Error(_)) => (rest, None),
        Err(e) => return Err(e),
    };

    let (rest, query) = match maybe_lpadded(preceded(char('|'), maybe_lpadded(query)))(rest) {
        Ok((rest, query)) => (rest, Some(query)),
        Err(nom::Err::Error(_)) => (rest, None),
        Err(e) => return Err(e),
    };

    let (rest, formatter) = match maybe_lpadded(preceded(char('|'), maybe_lpadded(formatter)))(rest)
    {
        Ok((rest, formatter)) => (rest, Some(formatter)),
        Err(nom::Err::Error(_)) => (rest, None),
        Err(e) => return Err(e),
    };

    Ok((
        rest,
        AST {
            decoder,
            mapper,
            query,
            formatter,
        },
    ))
}

fn decoder(input: Span) -> IResult<Decoder> {
    let (rest, try_parse_regex) = match char('/')(input) {
        Ok((rest, _)) => (rest, true),
        Err(nom::Err::Error(_)) => (input, false),
        Err(e) => return Err(e),
    };

    if try_parse_regex {
        // TODO: fix it! Less something less naive (e.g., escaped-strings-like parser).
        let end_pos = match find_unescaped(*rest, '/') {
            Some(end_pos) => end_pos,
            None => {
                return Err(nom::Err::Failure(ParseError::partial(
                    "regex",
                    "closing '/' symbol",
                    rest,
                )));
            }
        };

        let (rest, regex) = take::<usize, Span, ParseError>(end_pos + 1)(rest).unwrap();
        return Ok((
            rest,
            Decoder::Regex {
                regex: (*regex).to_owned(),
            },
        ));
    }

    let (rest, kind) = alt((tag_no_case("json"), tag_no_case("json")))(input)?;

    match kind.to_lowercase().as_str() {
        "json" => Ok((rest, Decoder::JSON)),
        _ => unimplemented!(),
    }
}

fn mapper(input: Span) -> IResult<Mapper> {
    let (rest, _) = tag_no_case("implement me!")(input)?;
    Ok((rest, Mapper {}))
}

fn query(input: Span) -> IResult<String> {
    let (rest, _) = tag_no_case("implement me!")(input)?;
    Ok((rest, "foobar".to_owned()))
}

fn formatter(input: Span) -> IResult<Formatter> {
    let (rest, kind) = alt((tag_no_case("to_json"), tag_no_case("to_json")))(input)?;

    match kind.to_lowercase().as_str() {
        "to_json" => Ok((rest, Formatter::JSON)),
        _ => unimplemented!(),
    }
}

fn find_unescaped(stack: &str, needle: char) -> Option<usize> {
    let mut armed = false;
    for (i, c) in stack.chars().enumerate() {
        if !armed && c == '\\' {
            armed = true;
            continue;
        }

        if !armed && c == needle {
            return Some(i);
        }

        if armed {
            armed = false;
        }
    }
    return None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_program() -> std::result::Result<(), String> {
        #[rustfmt::skip]
        let tests = [
            r#"//"#,
            r#"/foo/"#,
            r#"/foo\/bar/"#,
            r#"/.*(\\d+)foo\\s(\\w+).+/"#,
            r#"json"#,
            r#"json | to_json"#,
            r#"json| to_json"#,
            r#"json |to_json"#,
            r#"json|to_json"#,
            r#"/.*(\\d+)foo\\s(\\w+).+/ | to_json"#,
        ];

        for input in &tests {
            parse_program(input).map_err(|e| format!("Got {:?} while parsing {}", e, input))?;
        }
        Ok(())
    }
}
