use std::convert::TryFrom;
use std::fmt::Debug;
use std::num::ParseIntError;

use thiserror::Error;


#[derive(Error, Debug)]
#[error("Failed to parse string '{value}' into a version!")]
pub struct InvalidVersionStringError {
    value: String,
}

impl From<std::num::ParseIntError> for InvalidVersionStringError {
    fn from(error: ParseIntError) -> Self {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Version {
        Version { major, minor, patch}
    }
}

impl TryFrom<String> for Version {
    type Error = InvalidVersionStringError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        parse_version_string(value.as_str())
    }
}

impl TryFrom<&str> for Version {
    type Error = InvalidVersionStringError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse_version_string(value)
    }
}

fn parse_version_string(value: &str) -> Result<Version, InvalidVersionStringError> {
    let split: Vec<&str> = value.split('.').collect();
    Ok(Version {
        major: split.get(0).unwrap_or(&"0").parse::<u32>()?,
        minor: split.get(1).unwrap_or(&"0").parse::<u32>()?,
        patch: split.get(2).unwrap_or(&"0").parse::<u32>()?,
    })
}

pub fn format_bool(value: bool) -> String {
    if value {
        String::from("true")
    }
    else {
        String::from("false")
    }
}

pub trait HasBuilder {
    type Builder;
    fn builder() -> Self::Builder;
}

pub trait FastNearestMultiple: Copy + Sized {
    fn nearest_multiple(self, multiple: Self) -> Self;
}

impl FastNearestMultiple for u64 {

    /// Computes the nearest multiple of the specified number. Expects that the given multiple is a power of 2!
    #[inline(always)]
    fn nearest_multiple(self, multiple: Self) -> Self {
        debug_assert!(((multiple & (multiple - 1)) == 0), "{:?} is not a power of 2", multiple);
        (self + multiple - 1) & (!multiple + 1)
    }
}

#[cfg(test)]
mod tests {
    use hamcrest2::prelude::*;

    use crate::util::FastNearestMultiple;

    #[test]
    fn test_vk_version() {
        assert_that!(1, is(equal_to(1)));
    }

    #[test]
    fn test_nearest_multiple() {
        assert_that!(12u64.nearest_multiple(16), is(equal_to(16)));
        assert_that!(20u64.nearest_multiple(16), is(equal_to(32)));
        assert_that!(20u64.nearest_multiple(32), is(equal_to(32)));
        assert_that!(100u64.nearest_multiple(32), is(equal_to(128)));
    }

    #[test]
    #[should_panic]
    fn test_nearest_multiple_should_panic_if_multiple_is_not_a_power_of_two() {
        assert_that!(20u64.nearest_multiple(20), is(equal_to(32)));
    }
}
