//! Composed of two records, the record descriptor type and descryptor values
//! are separated in two record, for some reason.
//!
//! A descriptor id have the format 0xABBB, A is the kind, B is the id. The
//! kind could be:
//! 0..4 => , with "1 >> kind" is the data size.
//!
//! 4 => A variable data size, the next 2 bytes after the descritod type is the
//! size. But this need more investigation.
//!
//! 5 => End of the list, possibly only ID 0x003 is valid.
//!
//!
//! The order or descriptors seems to be irrelevant.

//TODO doc this

use byteorder::ByteOrder;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io::Result;

use crate::RecordHeader;

#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
pub enum FirmwareRecord {
    /// Empty firmware chunk. Some files include this if firmware_len = 0.
    EmptyChunk { id: u16 },
    /// Chunk of firmware data.
    Chunk { id: u16, data: Vec<u8> },
}

impl Display for FirmwareRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FirmwareRecord::EmptyChunk { id } => {
                write!(f, "FirmwareRecord::EmptyChunk {{ id: {} }}", id)
            }
            FirmwareRecord::Chunk { id, data } => write!(
                f,
                "FirmwareRecord::Chunk {{ id: {}, len: {} }}",
                id,
                data.len()
            ),
        }
    }
}

impl FirmwareRecord {
    pub fn new(data: Vec<u8>, id: u16) -> Self {
        if data.len() == 0 {
            FirmwareRecord::EmptyChunk { id }
        } else {
            FirmwareRecord::Chunk { id, data }
        }
    }
    pub fn len(&self) -> u16 {
        match self {
            FirmwareRecord::EmptyChunk { .. } => 0,
            FirmwareRecord::Chunk { data, .. } => data.len() as u16,
        }
    }
    pub const fn id(&self) -> u16 {
        match self {
            FirmwareRecord::EmptyChunk { id }
            | FirmwareRecord::Chunk { id, .. } => *id,
        }
    }
    pub fn data(&self) -> &[u8] {
        match self {
            FirmwareRecord::EmptyChunk { .. } => &[],
            FirmwareRecord::Chunk { data, .. } => data,
        }
    }
    pub fn record_to_raw<B: ByteOrder>(&self, data: &mut [u8]) -> Result<()> {
        //write header
        let next = RecordHeader::Unknown {
            id: self.id(),
            len: self.len(),
        }
        .to_raw::<B>(data)?;

        //write record body
        next[..self.len() as usize].copy_from_slice(self.data());

        Ok(())
    }
}
