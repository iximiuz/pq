use std::collections::HashSet;

use crate::error::{Error, Result};
use crate::query::parser::{ast::Expr as QueryExpr, expr::expr as query_expr};
use crate::utils::parse::{
    label_identifier, maybe_lpadded, separated_list, string_literal, IResult, ParseError, Span,
};

use nom::{
    branch::alt,
    bytes::complete::{tag_no_case, take},
    character::complete::{char, digit1},
    combinator::{map, value},
    sequence::preceded,
};

// TODO: make all attributes private, add constructors with validation logic, and proper getters.

#[derive(Debug)]
pub struct AST {
    pub decoder: Decoder,
    pub mapper: Option<Mapper>,
    pub query: Option<QueryExpr>,
    pub formatter: Option<Formatter>,
}

#[derive(Clone, Debug)]
pub enum Decoder {
    JSON,
    Regex { regex: String },
    // TODO:
    // CSV {
    //     header: Vec<String>,
    //     separator: String,
    // },
    // logfmt
    // scanf
    // Prometheus,
    // InfluxDB,
    // Nginx,
    // Nginx:combined,
    // Apache,
    // Envoy,
    // Redis
}

#[derive(Debug)]
pub struct Mapper {
    pub fields: Vec<MapperField>,
}

#[derive(Debug)]
pub struct MapperField {
    pub loc: FieldLoc,
    pub typ: FieldType,
    pub alias: Option<String>,
}

impl MapperField {
    pub fn end_name(&self) -> String {
        if let Some(ref alias) = self.alias {
            return alias.clone();
        }

        if let FieldLoc::Name(ref name) = self.loc {
            return name.clone();
        }

        if let FieldLoc::Position(pos) = self.loc {
            return format!("f{}", pos);
        }

        panic!("malformed field definition");
    }
}

#[derive(Debug)]
pub enum FieldLoc {
    Name(String),
    Position(usize),
}

#[derive(Clone, Debug)]
pub enum FieldType {
    Auto,
    Number,
    String,
    Const(String),
    Timestamp(Option<String>),
}

#[derive(Clone, Debug)]
pub enum Formatter {
    HumanReadable,
    JSON,
    PromAPI,
    // TODO:
    // PromQL,
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
            "a valid pq program must start from a known parser (supported parsers: regex /.../, json)"
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
    let (rest, decoder) = alt((decoder_regex, value(Decoder::JSON, tag_no_case("json"))))(input)?;
    Ok((rest, decoder))
}

fn decoder_regex(input: Span) -> IResult<Decoder> {
    let (rest, _) = char('/')(input)?;

    // TODO: fix it! Less something less naive (e.g., escaped-strings-like parser).
    if let Some(end_pos) = find_unescaped(*rest, '/') {
        let (rest, regex) = take::<usize, Span, ParseError>(end_pos)(rest)?;
        let (rest, _) = char('/')(rest)?;
        return Ok((
            rest,
            Decoder::Regex {
                regex: (*regex).replace(r#"\/"#, "/").to_owned(),
            },
        ));
    }

    Err(nom::Err::Failure(ParseError::partial(
        "regex",
        "closing '/' symbol",
        rest,
    )))
}

fn mapper(input: Span) -> IResult<Mapper> {
    let (rest, _) = tag_no_case("map")(input)?;
    let (rest, fields) = match maybe_lpadded(separated_list(
        '{',
        '}',
        ',',
        mapper_field,
        "map expression",
        "field definition (example: '.foo:str') or '}'",
    ))(rest)
    {
        Ok((rest, fields)) => (rest, fields),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "map expression",
                "comma-separated list of fields",
                rest,
            )))
        }
        Err(e) => return Err(e),
    };

    let mut count_timestamps = 0;
    let mut count_loc_by_name = 0;
    let mut count_loc_by_pos = 0;
    let mut end_names = HashSet::new();
    for field in fields.iter() {
        if let FieldType::Timestamp(_) = field.typ {
            count_timestamps += 1;
        }

        if !end_names.insert(field.end_name()) {
            return Err(nom::Err::Failure(ParseError::new(
                format!(
                    "ambiguous field name in map expression '{}'",
                    field.end_name()
                ),
                rest,
            )));
        }

        if let FieldType::Const(_) = field.typ {
            // noop for now
        } else {
            match field.loc {
                FieldLoc::Name(_) => count_loc_by_name += 1,
                FieldLoc::Position(_) => count_loc_by_pos += 1,
            }
        }
    }

    if fields.len() == 0 {
        return Err(nom::Err::Failure(ParseError::new(
            "map expression must have at least one field definition (example: .1:str as some_name)"
                .to_owned(),
            rest,
        )));
    }
    if count_timestamps > 1 {
        return Err(nom::Err::Failure(ParseError::new(
            "map expression cannot have more than one timestamp field definition".to_owned(),
            rest,
        )));
    }
    if count_loc_by_name > 0 && count_loc_by_pos > 0 {
        return Err(nom::Err::Failure(ParseError::new(
            "all field definition must be either position-based (.0, .1, etc) or name-based (.foo, .bar, etc)".to_owned(),
            rest,
        )));
    }

    Ok((rest, Mapper { fields }))
}

fn mapper_field(input: Span) -> IResult<MapperField> {
    alt((mapper_field_dynamic, mapper_field_const))(input)
}

// .0:ts "%Y-%m-%d" as time
// .1 as method
// .foo:num
// .qux:str as bar,
fn mapper_field_dynamic(input: Span) -> IResult<MapperField> {
    let (rest, _) = char('.')(input)?;

    let (rest, loc) = match alt((
        map(digit1, |d: Span| {
            FieldLoc::Position((*d).parse::<usize>().unwrap())
        }),
        map(label_identifier, |n| FieldLoc::Name(n)),
    ))(rest)
    {
        Ok((rest, loc)) => (rest, loc),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "map expression",
                "field position (number) or name (identifier)",
                rest,
            )));
        }
        Err(e) => return Err(e),
    };

    let (rest, typ) = match maybe_lpadded(mapper_field_type)(rest) {
        Ok((rest, typ)) => (rest, typ),
        Err(nom::Err::Error(_)) => (rest, FieldType::Auto),
        Err(e) => return Err(e),
    };

    let (rest, alias) = match maybe_lpadded(mapper_field_alias)(rest) {
        Ok((rest, alias)) => (rest, Some(alias)),
        Err(nom::Err::Error(_)) => (rest, None),
        Err(e) => return Err(e),
    };

    Ok((rest, MapperField { loc, typ, alias }))
}

fn mapper_field_type(input: Span) -> IResult<FieldType> {
    let (rest, _) = char(':')(input)?;

    let (rest, typ) = match maybe_lpadded(alt((
        value(FieldType::String, maybe_lpadded(tag_no_case("str"))),
        value(FieldType::Number, maybe_lpadded(tag_no_case("num"))),
        value(FieldType::Timestamp(None), maybe_lpadded(tag_no_case("ts"))),
    )))(rest)
    {
        Ok((rest, typ)) => (rest, typ),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "map expression",
                "type (str, num, or ts)",
                rest,
            )));
        }
        Err(e) => return Err(e),
    };

    if let FieldType::Timestamp(_) = typ {
        let (rest, format) = match maybe_lpadded(string_literal)(rest) {
            Ok((rest, format)) => (rest, Some(format)),
            Err(nom::Err::Error(_)) => (rest, None),
            Err(e) => return Err(e),
        };
        return Ok((rest, FieldType::Timestamp(format)));
    }

    Ok((rest, typ))
}

fn mapper_field_alias(input: Span) -> IResult<String> {
    let (rest, _) = tag_no_case("as ")(input)?;

    let (rest, alias) = match maybe_lpadded(label_identifier)(rest) {
        Ok((rest, alias)) => (rest, alias),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "map expression",
                "alias (identifier)",
                rest,
            )));
        }
        Err(e) => return Err(e),
    };

    Ok((rest, alias))
}

// extra_label: "value"
fn mapper_field_const(input: Span) -> IResult<MapperField> {
    let (rest, name) = label_identifier(input)?;

    let rest = match maybe_lpadded(char(':'))(rest) {
        Ok((rest, _)) => rest,
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "map expression",
                ":",
                rest,
            )));
        }
        Err(e) => return Err(e),
    };

    let (rest, value) = match maybe_lpadded(string_literal)(rest) {
        Ok((rest, value)) => (rest, value),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "map expression",
                "string literal",
                rest,
            )));
        }
        Err(e) => return Err(e),
    };

    Ok((
        rest,
        MapperField {
            loc: FieldLoc::Name(name),
            typ: FieldType::Const(value.to_owned()),
            alias: None,
        },
    ))
}

fn query(input: Span) -> IResult<QueryExpr> {
    let (rest, _) = tag_no_case("select ")(input)?;
    let (rest, expr) = match maybe_lpadded(query_expr(None))(rest) {
        Ok((rest, ast)) => (rest, ast),
        Err(nom::Err::Error(_)) => {
            return Err(nom::Err::Failure(ParseError::partial(
                "query",
                "query expression",
                rest,
            )));
        }
        Err(e) => return Err(e),
    };
    Ok((rest, expr))
}

fn formatter(input: Span) -> IResult<Formatter> {
    let (rest, fmt) = alt((
        value(Formatter::JSON, tag_no_case("to_json")),
        value(Formatter::PromAPI, tag_no_case("to_promapi")),
    ))(input)?;
    Ok((rest, fmt))
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
            r#"json | to_promapi"#,
            r#"/.*(\\d+)foo\\s(\\w+).+/ | to_json"#,
            r#"/.*(\\d+)foo\\s(\\w+).+/ | map {foo: "bar"} | to_json"#,
            r#"/.*(\\d+)foo\\s(\\w+).+/ | map {.0:str, .1:num as qux, .2:ts "%Y-%m-%d", foo: "bar"} | to_json"#,
            r#"/.*(\\d+)foo\\s(\\w+).+/ | map {.foo:str as bar, .qux:num, .ts:ts "%Y-%m-%d", abc: "42"} | to_json"#,
            r#"/.*(\\d+)foo\\s(\\w+).+/ | map {.foo:str as bar, .qux:num, .ts:ts "%Y-%m-%d", abc: "42"} | select {__name__=~"abc|foo"} / 9001 | to_json"#,
        ];

        for input in &tests {
            parse_program(input).map_err(|e| format!("Got {:?} while parsing {}", e, input))?;
        }
        Ok(())
    }

    #[test]
    fn test_regex_decoder() -> std::result::Result<(), String> {
        #[rustfmt::skip]
        let tests = [
            (r#"//"#, ""),
            (r#"/foo/"#, "foo"),
             (r#"/foo\/bar/"#, "foo/bar"),
        ];

        for (input, expected) in &tests {
            let ast =
                parse_program(input).map_err(|e| format!("Got {:?} while parsing {}", e, input))?;
            match ast.decoder {
                Decoder::Regex { regex: actual } => assert_eq!(*expected, actual),
                v => panic!("unexpected decoder {:?} while parsing {}", v, input),
            }
        }
        Ok(())
    }
}
