use std::convert::TryFrom;
use std::num::ParseIntError;

use regex::Regex;
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
    let re = Regex::new(r"^(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)$").unwrap();
    match re.captures(value) {
        Some(caps) => {
            let major = caps.name("major").map(|m| m.as_str()).unwrap_or("0").parse::<u32>()?;
            let minor = caps.name("minor").map(|m| m.as_str()).unwrap_or("0").parse::<u32>()?;
            let patch = caps.name("patch").map(|m| m.as_str()).unwrap_or("0").parse::<u32>()?;
            Ok(Version {
                major,
                minor,
                patch
            })
        },
        None => Err(InvalidVersionStringError { value: String::from(value)})
    }
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

#[cfg(test)]
mod tests {
    use hamcrest2::prelude::*;

    #[test]
    fn test_vk_version() {
        assert_that!(1, is(equal_to(1)));
    }
}
