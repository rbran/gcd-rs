use std::io::{Error, ErrorKind, Result};

use byteorder::ByteOrder;
use serde::{Deserialize, Serialize};

pub const ID: u16 = 6;

#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
pub enum DescriptorType {
    U8 { id: u16 },
    U16 { id: u16 },
    U32 { id: u16 },
    U64 { id: u16 },
    Other { id: u16, lenght: u16 },
    End, //only 0x5003 is valid, other value have unknown meaning
}

impl DescriptorType {
    pub fn from_raw<B: ByteOrder>(
        data: &[u8],
    ) -> Result<(&[u8], DescriptorType)> {
        if data.len() < 2 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Descriptor Type is bigger than the data available",
            ));
        }
        let value = B::read_u16(data);
        let kind = value >> 12 as u8;
        let id = value & 0x0fff;
        match kind {
            0 => Ok((&data[2..], DescriptorType::U8 { id })),
            1 => Ok((&data[2..], DescriptorType::U16 { id })),
            2 => Ok((&data[2..], DescriptorType::U32 { id })),
            3 => Ok((&data[2..], DescriptorType::U64 { id })),
            4 => {
                if data.len() < 4 {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Descriptor Type \"Other\" is missing the lenght",
                    ));
                }
                let lenght = B::read_u16(&data[2..]);
                Ok((&data[4..], DescriptorType::Other { id, lenght }))
            }
            5 => Ok((&data[2..], DescriptorType::End)),
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Descriptor Type has unknown value: {}", value),
            )),
        }
    }
    pub fn to_raw<'a, B: ByteOrder>(
        &self,
        data: &'a mut [u8],
    ) -> Result<&'a mut [u8]> {
        let data_len = self.len() as usize;
        if data.len() < data_len {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Descriptor Type buffer is to small",
            ));
        }
        B::write_u16(data, self.value());
        match self {
            DescriptorType::End
            | DescriptorType::U8 { .. }
            | DescriptorType::U16 { .. }
            | DescriptorType::U32 { .. }
            | DescriptorType::U64 { .. } => {}
            DescriptorType::Other { lenght, .. } => {
                B::write_u16(&mut data[2..], *lenght)
            }
        }
        Ok(&mut data[data_len..])
    }
    pub const fn kind(&self) -> u8 {
        match self {
            DescriptorType::U8 { .. } => 0,
            DescriptorType::U16 { .. } => 1,
            DescriptorType::U32 { .. } => 2,
            DescriptorType::U64 { .. } => 3,
            DescriptorType::Other { .. } => 4,
            DescriptorType::End => 5,
        }
    }
    pub const fn id(&self) -> u16 {
        match self {
            DescriptorType::U8 { id }
            | DescriptorType::U16 { id }
            | DescriptorType::U32 { id }
            | DescriptorType::U64 { id }
            | DescriptorType::Other { id, .. } => *id,
            DescriptorType::End => 3,
        }
    }
    pub const fn value(&self) -> u16 {
        ((self.kind() as u16) << 12) | self.id()
    }
    pub const fn len(&self) -> u16 {
        match self {
            DescriptorType::U8 { .. }
            | DescriptorType::U16 { .. }
            | DescriptorType::U32 { .. }
            | DescriptorType::U64 { .. }
            | DescriptorType::End => 2,
            DescriptorType::Other { .. } => 4,
        }
    }
    pub const fn data_len(&self) -> u16 {
        match self {
            DescriptorType::U8 { .. } => 1,
            DescriptorType::U16 { .. } => 2,
            DescriptorType::U32 { .. } => 4,
            DescriptorType::U64 { .. } => 8,
            DescriptorType::Other { lenght, .. } => *lenght,
            DescriptorType::End => 0,
        }
    }
}
