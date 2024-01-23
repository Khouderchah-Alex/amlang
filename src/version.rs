use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

use std::fmt;
use std::str::FromStr;

use crate::Error;


/// Semantics of a version.
#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Version {
    major: usize,
    minor: usize,
    patch: usize,
}

/// Version collapsed into a string. Can only be created from a Version.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VersionString(String);

impl Version {
    pub fn new(major: usize, minor: usize, patch: usize) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}


impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref COMPS: Regex = Regex::new(r"^(\d+)\.(\d+)\.(\d+)$").unwrap();
        }

        if let Some(cap) = COMPS.captures(s) {
            let usize_err = |e: std::num::ParseIntError| {
                return Error::adhoc("ParseInt", e.to_string());
            };

            let major = cap.get(1).unwrap().as_str().parse().map_err(usize_err)?;
            let minor = cap.get(2).unwrap().as_str().parse().map_err(usize_err)?;
            let patch = cap.get(3).unwrap().as_str().parse().map_err(usize_err)?;
            Ok(Self {
                major,
                minor,
                patch,
            })
        } else {
            Err(Error::adhoc("ParseVersion", s.to_string()))
        }
    }
}

impl From<VersionString> for Version {
    fn from(s: VersionString) -> Self {
        // Under the assumption that VersionString is created only by Version,
        // this unwrap should be ok.
        s.0.parse().unwrap()
    }
}

impl From<Version> for VersionString {
    fn from(v: Version) -> Self {
        Self(v.to_string())
    }
}
