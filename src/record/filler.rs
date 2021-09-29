use byteorder::ByteOrder;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind, Result};

use crate::{RecordHeader, RECORD_HEADER_LEN};

pub const ID: u16 = 2;
#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FillerRecord {
    Zeros(u16),
}

impl Display for FillerRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FillerRecord::Zeros(len) => {
                write!(f, "FillerRecord:Zeros({})", len)
            }
        }
    }
}

impl FillerRecord {
    pub fn new(data: &[u8]) -> Result<Self> {
        if data.iter().find(|&x| *x != 0).is_some() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Invalid Filler Record value",
            ));
        }
        Ok(FillerRecord::Zeros(data.len() as u16))
    }
    pub const fn len(&self) -> u16 {
        match self {
            FillerRecord::Zeros(len) => *len,
        }
    }
    pub fn header(&self) -> RecordHeader {
        RecordHeader::Filler(self.len())
    }
    pub fn record_to_raw<B: ByteOrder>(&self, data: &mut [u8]) -> Result<()> {
        //write header
        self.header().to_raw::<B>(data)?;
        data[RECORD_HEADER_LEN..RECORD_HEADER_LEN + self.len() as usize]
            .fill(0);

        Ok(())
    }
}
