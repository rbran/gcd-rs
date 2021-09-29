use byteorder::ByteOrder;
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind, Result};

use crate::Version;

use super::descriptor_type::DescriptorType;

pub const ID: u16 = 7;

#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
pub enum DescriptorData {
    U8 { id: u16, data: u8 },
    U16 { id: u16, data: u16 },
    U32 { id: u16, data: u32 },
    U64 { id: u16, data: u64 },
    Other { id: u16, data: Vec<u8> },
    End, //only 0x5003 is valid, other value have unknown meaning
}

impl DescriptorData {
    pub fn from_raw<'a, 'b, B: ByteOrder>(
        descriptor_type: &'a DescriptorType,
        data: &'b [u8],
    ) -> Result<(&'b [u8], DescriptorData)> {
        let len = descriptor_type.data_len() as usize;
        if data.len() < len {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Descriptor Data is bigger than the data available",
            ));
        }
        let descriptor = match descriptor_type {
            DescriptorType::U8 { id } => DescriptorData::U8 {
                id: *id,
                data: data[0],
            },
            DescriptorType::U16 { id } => DescriptorData::U16 {
                id: *id,
                data: B::read_u16(data),
            },
            DescriptorType::U32 { id } => DescriptorData::U32 {
                id: *id,
                data: B::read_u32(data),
            },
            DescriptorType::U64 { id } => DescriptorData::U64 {
                id: *id,
                data: B::read_u64(data),
            },
            DescriptorType::Other { id, .. } => DescriptorData::Other {
                id: *id,
                data: data[..len].to_vec(),
            },
            DescriptorType::End => DescriptorData::End,
        };
        Ok((&data[len..], descriptor))
    }
    pub fn to_raw<'a, B: ByteOrder>(
        &self,
        buf: &'a mut [u8],
    ) -> Result<&'a mut [u8]> {
        let len = self.len() as usize;
        if buf.len() < len {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Descriptor Data buffer is to small",
            ));
        }
        match self {
            DescriptorData::U8 { data, .. } => buf[0] = *data,
            DescriptorData::U16 { data, .. } => B::write_u16(buf, *data),
            DescriptorData::U32 { data, .. } => B::write_u32(buf, *data),
            DescriptorData::U64 { data, .. } => B::write_u64(buf, *data),
            DescriptorData::Other { data, .. } => buf.copy_from_slice(data),
            DescriptorData::End => {}
        }
        Ok(&mut buf[len..])
    }
    pub fn descriptor_type(&self) -> DescriptorType {
        match self {
            DescriptorData::U8 { id, .. } => DescriptorType::U8 { id: *id },
            DescriptorData::U16 { id, .. } => DescriptorType::U16 { id: *id },
            DescriptorData::U32 { id, .. } => DescriptorType::U32 { id: *id },
            DescriptorData::U64 { id, .. } => DescriptorType::U64 { id: *id },
            DescriptorData::Other { id, data } => DescriptorType::Other {
                id: *id,
                lenght: data.len() as u16,
            },
            DescriptorData::End => DescriptorType::End,
        }
    }
    pub fn len(&self) -> u16 {
        self.descriptor_type().data_len()
    }
    pub const fn decode(&self) -> Option<DescriptorDecoded> {
        DescriptorDecoded::decode(self)
    }
}

#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
pub enum DescriptorDecoded {
    End,
    HWID(u16),
    XorKey(u8),
    FirmwareId(u16),
    FirmwareLen(u32),
    FirmwareAddr(u32),
    VersionSw(Version),
    VersionRemote(Version),
    VersionId12(Version),
    VersionId20(Version),
    Firmware2000P1Len(u32),
    Firmware2000P2Len(u32),
    Firmware2000P3Len(u32),
}

impl DescriptorDecoded {
    pub const fn decode(src: &DescriptorData) -> Option<DescriptorDecoded> {
        match src {
            DescriptorData::End => Some(DescriptorDecoded::End),
            DescriptorData::U8 { id: 10, data } => {
                Some(DescriptorDecoded::XorKey(*data))
            }
            DescriptorData::U16 { id: 9, data } => {
                Some(DescriptorDecoded::HWID(*data))
            }
            DescriptorData::U16 { id: 10, data } => {
                Some(DescriptorDecoded::FirmwareId(*data))
            }
            DescriptorData::U16 { id: 12, data } => {
                Some(DescriptorDecoded::VersionId12(Version::new_raw(*data)))
            }
            DescriptorData::U16 { id: 13, data } => {
                Some(DescriptorDecoded::VersionSw(Version::new_raw(*data)))
            }
            DescriptorData::U16 { id: 20, data } => {
                Some(DescriptorDecoded::VersionId20(Version::new_raw(*data)))
            }
            DescriptorData::U16 { id: 21, data } => {
                Some(DescriptorDecoded::VersionRemote(Version::new_raw(*data)))
            }
            DescriptorData::U32 { id: 21, data } => {
                Some(DescriptorDecoded::FirmwareLen(*data))
            }
            DescriptorData::U32 { id: 23, data } => {
                Some(DescriptorDecoded::Firmware2000P1Len(*data))
            }
            DescriptorData::U32 { id: 24, data } => {
                Some(DescriptorDecoded::Firmware2000P2Len(*data))
            }
            DescriptorData::U32 { id: 25, data } => {
                Some(DescriptorDecoded::Firmware2000P3Len(*data))
            }
            DescriptorData::U32 { id: 26, data } => {
                Some(DescriptorDecoded::FirmwareAddr(*data))
            }
            DescriptorData::U8 { .. } => None,
            DescriptorData::U16 { .. } => None,
            DescriptorData::U32 { .. } => None,
            DescriptorData::U64 { .. } => None,
            DescriptorData::Other { .. } => None,
        }
    }
}
