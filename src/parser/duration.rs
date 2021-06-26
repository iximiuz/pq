use std::convert::TryFrom;
use std::time::Duration;

use nom::{branch::alt, bytes::complete::tag, character::complete::digit1};

use super::result::{IResult, ParseError, Span};
use crate::error::{Error, Result};

pub fn parse_duration(s: &str) -> Result<Duration> {
    match duration(Span::new(s)) {
        Ok((_, d)) => Ok(d),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => Err(Error::from(e.message())),
        _ => unreachable!(),
    }
}

/// Parse Go-like duration string: `2s`, `1y3w5d7h9m`.
/// - Only positive durations.
/// - No fractional units.
/// - Units are always ordered from longest to shortest.
pub(super) fn duration(input: Span) -> IResult<Duration> {
    let (rest, duration) = duration_inner(input, Unit::Year)?;

    if duration.eq(&Duration::from_millis(0)) {
        return Err(nom::Err::Failure(ParseError::new(
            "duration must be greater than 0".to_owned(),
            input,
        )));
    }

    Ok((rest, duration))
}

enum Unit {
    Millisecond,
    Second, // 1000 milliseconds
    Minute, // 60 seconds
    Hour,   // 60 minutes
    Day,    // 24 hours
    Week,   // 7 days
    Year,   // 365 days, always
}

impl Unit {
    fn milliseconds(&self) -> u64 {
        use Unit::*;
        match self {
            Millisecond => 1,
            Second => 1000,
            Minute => 60 * 1000,
            Hour => 60 * 60 * 1000,
            Day => 24 * 60 * 60 * 1000,
            Week => 7 * 24 * 60 * 60 * 1000,
            Year => 365 * 24 * 60 * 60 * 1000,
        }
    }

    fn descendant(&self) -> Option<Self> {
        use Unit::*;
        match self {
            Millisecond => None,
            Second => Some(Millisecond),
            Minute => Some(Second),
            Day => Some(Hour),
            Hour => Some(Minute),
            Week => Some(Day),
            Year => Some(Week),
        }
    }
}

impl std::convert::TryFrom<&str> for Unit {
    type Error = Error;

    fn try_from(u: &str) -> Result<Self> {
        use Unit::*;

        match u {
            "y" => Ok(Year),
            "w" => Ok(Week),
            "d" => Ok(Day),
            "h" => Ok(Hour),
            "m" => Ok(Minute),
            "s" => Ok(Second),
            "ms" => Ok(Millisecond),
            _ => Err(Error::new("Unknown duration unit")),
        }
    }
}

fn duration_inner(input: Span, max_allowed_unit: Unit) -> IResult<Duration> {
    let (rest, multiplier) = digit1(input)?;

    let (rest, unit) = alt((
        tag("ms"),
        tag("s"),
        tag("m"),
        tag("h"),
        tag("d"),
        tag("w"),
        tag("y"),
    ))(rest)?;

    let unit = Unit::try_from(*unit).unwrap();
    if unit.milliseconds() > max_allowed_unit.milliseconds() {
        return Err(nom::Err::Failure(ParseError::new(
            "invalid duration literal".to_owned(),
            input,
        )));
    }

    let multiplier = multiplier.parse::<u32>().unwrap();
    let duration = Duration::from_millis(unit.milliseconds())
        .checked_mul(multiplier)
        .expect("duration overflow occurred");

    if let Some(next_unit) = unit.descendant() {
        let (rest, more_duration) = match duration_inner(rest, next_unit) {
            Ok((rest, more_duration)) => (rest, more_duration),
            Err(nom::Err::Error(_)) => (rest, Duration::from_millis(0)),
            Err(e) => return Err(e),
        };
        Ok((
            rest,
            duration
                .checked_add(more_duration)
                .expect("duration overflow occurred"),
        ))
    } else {
        Ok((rest, duration))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: u64 = 1000;
    const MINUTE: u64 = 60 * 1000;
    const HOUR: u64 = 60 * 60 * 1000;
    const DAY: u64 = 24 * 60 * 60 * 1000;
    const WEEK: u64 = 7 * 24 * 60 * 60 * 1000;
    const YEAR: u64 = 365 * 24 * 60 * 60 * 1000;

    #[test]
    fn test_valid_duration() -> std::result::Result<(), nom::Err<ParseError<'static>>> {
        #[rustfmt::skip]
        let tests = [
            ("1ms", Duration::from_millis(1)),
            ("10s", Duration::from_millis(10000)),
            ("0s500ms", Duration::from_millis(500)),
            ("5s999ms", Duration::from_millis(5999)),
            ("1y2w3d4h5m6s7ms", Duration::from_millis(YEAR + 2 * WEEK + 3 * DAY + 4 * HOUR + 5 * MINUTE + 6 * SECOND + 7)),
        ];

        for (input, expected_duration) in &tests {
            let (_, actual_duration) = duration(Span::new(input))?;
            assert_eq!(
                expected_duration, &actual_duration,
                "while parsing {}",
                input
            );
        }
        Ok(())
    }

    #[test]
    fn test_invalid_duration() {
        #[rustfmt::skip]
        let tests = [
            "foo",
            "0",
            "0ms",
            "1ns",
            "0s0ms",
            "10m2h",
        ];

        for input in &tests {
            let ret = duration(Span::new(input));
            assert!(
                ret.is_err(),
                "Expected error, got {:?} while parsing {}",
                ret,
                input
            );
        }
    }
}
