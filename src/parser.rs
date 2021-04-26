extern crate nom;

use nom::{bytes::streaming::take, IResult};

pub fn take4(input: &str) -> IResult<&str, &str> {
    take(4u8)(input)
}
