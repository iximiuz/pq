use crate::error::{Error, Result};
use crate::utils::parse::{maybe_lpadded, IResult, ParseError, Span};

use nom::{
    branch::alt,
    bytes::complete::{tag_no_case, take},
    character::complete::char,
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
    let (rest, decoder) = match decoder(input) {
        Ok((rest, decoder)) => (rest, decoder),
        Err(nom::Err::Error(_)) => return Err(nom::Err::Failure(ParseError::new(
            "a valid pq program must start from a known parser (supported parsers - regex, JSON)"
                .to_owned(),
            input,
        ))),
        Err(e) => return Err(e),
    };

    let (rest, try_parse_formatter) = match maybe_lpadded(char('|'))(rest) {
        Ok((rest, _)) => (rest, true),
        Err(nom::Err::Error(_)) => (rest, false),
        Err(e) => return Err(e),
    };

    if !try_parse_formatter {
        return Ok((
            rest,
            AST {
                decoder,
                mapper: None,
                query: None,
                formatter: None,
            },
        ));
    }

    let (rest, formatter) = match maybe_lpadded(formatter)(rest) {
        Ok((rest, formatter)) => (rest, formatter),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "program",
                "formatter (to_json is the only one supported at the moment)",
                rest,
            )))
        }
        Err(e) => return Err(e),
    };

    return Ok((
        rest,
        AST {
            decoder,
            mapper: None,
            query: None,
            formatter: Some(formatter),
        },
    ));
}

fn formatter(input: Span) -> IResult<Formatter> {
    let (rest, kind) = alt((tag_no_case("to_json"), tag_no_case("to_json")))(input)?;

    match kind.to_lowercase().as_str() {
        "to_json" => Ok((rest, Formatter::JSON)),
        _ => unimplemented!(),
    }
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
            "//",
            "/foo/",
            "/.*(\\d+)foo\\s(\\w+).+/",
            "json",
            "json | to_json",
            "json| to_json",
            "json |to_json",
            "json|to_json",
            "/.*(\\d+)foo\\s(\\w+).+/ | to_json",
        ];

        for input in &tests {
            parse_program(input).map_err(|e| format!("Got {:?} while parsing {}", e, input))?;
        }
        Ok(())
    }
}
