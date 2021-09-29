use byteorder::ByteOrder;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io::Result;

use crate::RecordHeader;

#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
pub enum TextRecord {
    Simple(String),
    Blob(Vec<u8>),
}

impl Display for TextRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TextRecord::Simple(x) => write!(f, "TextRecord:Simple({})", x),
            TextRecord::Blob(x) => {
                write!(f, "TextRecord:Blob(len: {})", x.len())
            }
        }
    }
}

pub const ID: u16 = 5;

impl TextRecord {
    pub fn new<F: std::io::Read>(file: &mut F, lenght: u16) -> Result<Self> {
        let mut data = vec![0; lenght as usize];
        file.read_exact(&mut data)?;
        match core::str::from_utf8(&data) {
            Ok(_) => Ok(TextRecord::Simple(unsafe {
                //allowed because the check was done on "core::str::from_utf8"
                String::from_utf8_unchecked(data)
            })),
            Err(_) => Ok(TextRecord::Blob(data)),
        }
    }
    pub fn len(&self) -> u16 {
        match self {
            TextRecord::Simple(data) => data.len() as u16,
            TextRecord::Blob(data) => data.len() as u16,
        }
    }
    pub fn value(&self) -> &[u8] {
        match self {
            TextRecord::Simple(x) => x.as_bytes(),
            TextRecord::Blob(x) => &x,
        }
    }
    pub fn record_to_raw<B: ByteOrder>(&self, data: &mut [u8]) -> Result<()> {
        //write header
        let next = RecordHeader::Text(self.len()).to_raw::<B>(data)?;
        next[..self.len() as usize].copy_from_slice(self.value());
        Ok(())
    }
}
