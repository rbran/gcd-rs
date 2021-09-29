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

use crate::{RecordHeader, RECORD_HEADER_LEN};
use byteorder::ByteOrder;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind, Result};

pub mod descriptor_data;
pub mod descriptor_type;

use descriptor_data::DescriptorData;
use descriptor_type::DescriptorType;

#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
pub enum DescriptorTypeRecord {
    Simple(Vec<DescriptorType>),
}

impl Display for DescriptorTypeRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DescriptorTypeRecord::Simple(x) => {
                write!(f, "DescriptorTypeRecord:Simple(len : {})", x.len())
            }
        }
    }
}

impl DescriptorTypeRecord {
    pub fn new<F, B>(file: &mut F, lenght: u16) -> Result<Self>
    where
        F: std::io::Read,
        B: ByteOrder,
    {
        if lenght % 2 != 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Record Descriptor type size need to be multiple of 2",
            ));
        }

        let mut data = vec![0u8; lenght as usize];
        file.read_exact(&mut data)?;

        // Obs for each Other sized, we allocate 2 bytes more then necessary.
        // Is very rare to have a Other sized, so the shrink is realy necessary?
        let mut descriptors = Vec::with_capacity(lenght as usize / 2);

        let mut current = data.as_slice();
        while current.len() != 0 {
            let (next, descriptor_type) =
                DescriptorType::from_raw::<B>(current)?;
            descriptors.push(descriptor_type);
            current = next;
        }
        Ok(DescriptorTypeRecord::Simple(descriptors))
    }
    pub fn len(&self) -> usize {
        match self {
            DescriptorTypeRecord::Simple(descs) => descs.len(),
        }
    }
    pub fn data_len(&self) -> u16 {
        match self {
            DescriptorTypeRecord::Simple(descs) => {
                descs.iter().map(|x| x.data_len()).sum::<u16>()
            }
        }
    }
    pub fn iter(&self) -> std::slice::Iter<DescriptorType> {
        match self {
            DescriptorTypeRecord::Simple(descs) => descs.iter(),
        }
    }
}

impl Default for DescriptorTypeRecord {
    fn default() -> Self {
        Self::Simple(vec![])
    }
}

#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
pub enum DescriptorRecord {
    Simple(Vec<DescriptorData>),
}
impl Display for DescriptorRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DescriptorRecord::Simple(x) => {
                write!(f, "DescriptorRecord:Simple(len : {})", x.len())
            }
        }
    }
}

impl DescriptorRecord {
    pub fn new<F, B>(
        file: &mut F,
        lenght: u16,
        desc_type: &DescriptorTypeRecord,
    ) -> Result<Self>
    where
        F: std::io::Read,
        B: ByteOrder,
    {
        // Check if Descriptor Type record expect this data size
        if desc_type.data_len() != lenght {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Record Descriptor data is Invalid/Unexpected",
            ));
        }

        //read the descriptor data
        let mut data = vec![0u8; lenght as usize];
        file.read_exact(&mut data)?;

        let mut current = data.as_slice();
        let descriptors = desc_type
            .iter()
            .map(|x| {
                let (next, desc_data) =
                    DescriptorData::from_raw::<B>(x, current)?;
                current = next;
                Ok(desc_data)
            })
            .collect::<Result<_>>()?;

        if current.len() != 0 {
            panic!("Programing Error on Descriptor Data parsing");
        }

        Ok(DescriptorRecord::Simple(descriptors))
    }
    pub fn iter(&self) -> std::slice::Iter<DescriptorData> {
        match self {
            DescriptorRecord::Simple(descs) => descs.iter(),
        }
    }
    pub fn record_type_len(&self) -> u16 {
        match self {
            DescriptorRecord::Simple(x) => {
                x.iter().map(|x| x.descriptor_type().len()).sum()
            }
        }
    }
    pub fn record_data_len(&self) -> u16 {
        match self {
            DescriptorRecord::Simple(x) => x.iter().map(|x| x.len()).sum(),
        }
    }
    pub fn record_type_to_raw<'a, B: ByteOrder>(
        &self,
        data: &'a mut [u8],
    ) -> Result<&'a mut [u8]> {
        //write header
        RecordHeader::DescriptorType(self.record_type_len())
            .to_raw::<B>(data)?;

        //write record body
        let mut current = &mut data[RECORD_HEADER_LEN..];
        for desc in self.iter() {
            current = desc.descriptor_type().to_raw::<B>(current)?;
        }

        Ok(current)
    }

    pub fn record_data_to_raw<'a, B: ByteOrder>(
        &self,
        data: &'a mut [u8],
    ) -> Result<&'a mut [u8]> {
        //write header
        RecordHeader::DescriptorData(self.record_data_len())
            .to_raw::<B>(data)?;

        //write record body
        let mut current = &mut data[RECORD_HEADER_LEN..];
        for desc in self.iter() {
            current = desc.to_raw::<B>(current).unwrap();
        }

        Ok(current)
    }
}
