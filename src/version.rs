//! Version represent some software version.
//!
//! Is composed of a major and minor values. It could be represented as
//! u8 or u16.
//!
//! The minor value is know to be on the range 0..100
//!
//! The major value is know to be on the range 0..2 in u8 format, or 0..65334 in
//! u16 format.
//!
//! The version is represented in decimal, the two least significant values
//! represent the minor, the rest represent the major. Eg: 380 (0x17c) result in
//! major 3 and minor 80, v3.80.
//!
//! The value 0xffff seems to be reserved. Possibly representing an Null for
//! the version value, if forced to print, it will simply print "0.0".

use serde::{Deserialize, Serialize};
use std::fmt;

/// Can be created from/to a u8 or u16 values.
#[derive(Debug, PartialEq, Hash, Eq, Copy, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Version {
    /// No version available
    None,
    /// Simple version format {major}.{minor}, eg: major = 3, minor = 80: v3.80
    Simple { major: u16, minor: u8 },
    // reserved for future version formats
}

impl Version {
    pub const fn new_raw(value: u16) -> Self {
        match value {
            0xffff => Version::None,
            x => Version::Simple {
                major: x / 100,
                minor: (x % 100) as u8,
            },
        }
    }

    pub const fn new(major: u16, minor: u8) -> Self {
        Version::Simple { major, minor }
    }

    pub const fn value(&self) -> u16 {
        match self {
            Version::None => 0xffff,
            Version::Simple { major, minor } => (*major * 100) + *minor as u16,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Version::None => write!(f, "0.0"),
            Version::Simple { major, minor } => {
                write!(f, "{}.{}", major, minor)
            }
        }
    }
}

impl From<u16> for Version {
    fn from(x: u16) -> Self {
        Version::new_raw(x)
    }
}

impl From<u8> for Version {
    fn from(x: u8) -> Self {
        Version::new_raw(x as u16)
    }
}

impl From<Version> for u16 {
    fn from(x: Version) -> u16 {
        x.value()
    }
}
