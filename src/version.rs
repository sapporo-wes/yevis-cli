use anyhow::{anyhow, Error, Result};
use std::cmp::{Ord, Ordering, PartialOrd};
use std::str::FromStr;
use std::string::ToString;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Version {
    major: usize,
    minor: usize,
    patch: usize,
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut version_parts = s.split('.');
        let major = version_parts
            .next()
            .ok_or_else(|| anyhow!("Failed to parse major version"))?
            .parse()?;
        let minor = version_parts
            .next()
            .ok_or_else(|| anyhow!("Failed to parse minor version"))?
            .parse()?;
        let patch = version_parts
            .next()
            .ok_or_else(|| anyhow!("Failed to parse patch version"))?
            .parse()?;
        Ok(Version {
            major,
            minor,
            patch,
        })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => match self.minor.cmp(&other.minor) {
                Ordering::Equal => match self.patch.cmp(&other.patch) {
                    Ordering::Equal => Ordering::Equal,
                    ordering => ordering,
                },
                ordering => ordering,
            },
            ordering => ordering,
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl ToString for Version {
    fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}
