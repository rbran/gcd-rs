//! The first data containing record.
//!
//! There are two known variations, the PartNumber (9 bytes) and HwID (2 bytes).

use byteorder::ByteOrder;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind, Result};

use crate::{RecordHeader, RECORD_HEADER_LEN};

pub const DEFAULT_HWID: u16 = 0x0037;
//const DEFAULT_PART_NUMBER: u128 = "010-10037-00".parse().data();
pub const DEFAULT_PART_NUMBER: u128 = 0x41140D4504135CD410;

pub const ID: u16 = 3;
/// Only two variations are known, 9 bytes for PartNumber and 2 bytes for HwId.
#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
pub enum MainRecord {
    /// The only know value is "010-10037-00".
    DefaultPartNumber,
    /// The only know value is 0x0037.
    DefaultHWID,
}

impl Display for MainRecord {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MainRecord::DefaultPartNumber => {
                write!(f, "MainRecord::DefaultPartNumber")
            }
            MainRecord::DefaultHWID => write!(f, "MainRecord::DefaultHWID"),
        }
    }
}

impl MainRecord {
    pub fn new<F, B>(file: &mut F, lenght: u16) -> Result<Self>
    where
        F: byteorder::ReadBytesExt,
        B: ByteOrder,
    {
        Ok(match lenght {
            9 => {
                let pn = file.read_uint128::<B>(9)?;
                if pn == DEFAULT_PART_NUMBER {
                    MainRecord::DefaultPartNumber
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Invalid/Unknown MainRecord PartNumber",
                    ));
                }
            }
            2 => {
                let hwid = file.read_u16::<B>()?;
                if hwid != DEFAULT_HWID {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Invalid/Unknown MainRecord HWID",
                    ));
                }
                MainRecord::DefaultHWID
            }
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid/Unknown Main Record",
                ))
            }
        })
    }

    pub const fn len(&self) -> u16 {
        match self {
            MainRecord::DefaultPartNumber => 9,
            MainRecord::DefaultHWID => 2,
        }
    }
    pub fn record_to_raw<B: ByteOrder>(&self, data: &mut [u8]) -> Result<()> {
        //write header
        RecordHeader::MainHeader(self.len()).to_raw::<B>(data)?;
        match self {
            MainRecord::DefaultPartNumber => B::write_uint128(
                &mut data[RECORD_HEADER_LEN..],
                DEFAULT_PART_NUMBER,
                9,
            ),
            MainRecord::DefaultHWID => {
                B::write_u16(&mut data[RECORD_HEADER_LEN..], DEFAULT_HWID)
            }
        }

        Ok(())
    }
}
