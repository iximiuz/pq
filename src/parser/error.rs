use nom::error::{ErrorKind, ParseError};

#[derive(Debug, PartialEq)]
pub struct MyError {
    pub message: String,
    // pos
    // expected token
    // name of the failed custom parser
}

impl<I> ParseError<I> for MyError {
    fn from_error_kind(_input: I, _kind: ErrorKind) -> Self {
        Self {
            message: "from_error_kind".into(),
        }
    }

    fn append(_input: I, _kind: ErrorKind, other: Self) -> Self {
        other
    }

    fn from_char(_input: I, _c: char) -> Self {
        Self {
            message: "from_char".into(),
        }
    }
}
