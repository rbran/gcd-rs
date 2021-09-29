use byteorder::ByteOrder;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind, Result};

use crate::RecordHeader;
use crate::RECORD_HEADER_LEN;

#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ChecksumRecord {
    Simple,
}

impl Display for ChecksumRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ChecksumRecord::Simple => write!(f, "ChecksumRecord:Simple"),
        }
    }
}

pub const ID: u16 = 1;
pub const LEN: u16 = 1;
impl ChecksumRecord {
    pub fn new(data: &[u8], checksum: u8) -> Result<Self> {
        if data.len() != 1 || checksum != 0 {
            Err(Error::new(
                ErrorKind::InvalidInput,
                "Invalid Checksum Value",
            ))
        } else {
            Ok(ChecksumRecord::Simple)
        }
    }
    pub const fn len(&self) -> u16 {
        match self {
            ChecksumRecord::Simple => LEN,
        }
    }
    pub fn record_to_raw<B: ByteOrder>(
        data: &mut [u8],
        checksum: u8,
    ) -> Result<()> {
        //write header
        RecordHeader::Checksum.to_raw::<B>(data)?;
        let value = data[..RECORD_HEADER_LEN]
            .iter()
            .fold(checksum, |acc, &x| x.wrapping_add(acc));

        //write record body
        data[RECORD_HEADER_LEN] = value.wrapping_neg();

        Ok(())
    }
}
