use super::ast::AST;
use super::expr::expr;
use super::result::{ParseError, ParseResult, Span};
use crate::error::{Error, Result};

pub fn parse_query(input: &str) -> Result<AST> {
    let (rest, ex) = match expr(Span::new(input)) {
        Ok((r, ParseResult::Complete(m))) => (r, m),
        Ok(res) => return Err(Error::from(err_msg_partial_result(res))),
        Err(nom::Err::Error(e)) => return Err(Error::from(err_msg_parse_error(e))),
        Err(nom::Err::Failure(e)) => return Err(Error::from(err_msg_parse_error(e))),
        Err(nom::Err::Incomplete(_)) => unreachable!(),
    };
    assert!(rest.len() == 0);
    Ok(AST::new(ex))
}

fn err_msg_partial_result<T>((input, partial): (Span, ParseResult<T>)) -> String {
    let (wherein, expected) = match partial {
        ParseResult::Partial(w, e) => (w, e),
        _ => panic!("partial_result_error_message() can be used only with ParseResult::Partial enum variant"),
    };

    fn unexpected(found: &str) -> String {
        match found {
            "" => String::from("EOF"),
            v => format!("\"{}\"", v),
        }
    }

    format!(
        "{}:{}: parse error: unexpected '{}' in {}, expected {}",
        input.location_line(),
        input.location_offset(),
        unexpected(*input),
        wherein,
        expected,
    )
}

fn err_msg_parse_error(err: ParseError) -> String {
    format!(
        "{}:{}: parse error: {}",
        err.line(),
        err.offset(),
        err.message()
    )
}
