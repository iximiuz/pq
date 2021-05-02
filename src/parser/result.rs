use nom;

use super::error::MyError;

pub type IResult<I, O> = nom::IResult<I, O, MyError>;
