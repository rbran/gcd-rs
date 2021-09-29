pub mod composer;
pub mod parser;

use byteorder::ByteOrder;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind, Result};

mod version;
pub use version::Version;

mod part_number;
pub use part_number::PartNumber;

pub mod record;
use record::main::MainRecord;

use record::checksum;
use record::checksum::ChecksumRecord;
use record::descriptor;
use record::descriptor::DescriptorRecord;
use record::filler;
use record::filler::FillerRecord;
use record::firmware::FirmwareRecord;
use record::main;
use record::text;

use self::record::text::TextRecord;

const RECORD_HEADER_LEN: usize = 4;

/// Alias for the only know Endian used by GCD files.
///
/// The only know GCD files are encoded using LittleEndian. But there is nothing
/// requiring the file format to never use BigEndian
///
/// Functions in this lib accept the user to specify the Endian as a future
/// proof.
pub type GcdDefaultEndian = byteorder::LE;

/// Known Record Headers, based on the current knowledge.
#[derive(Debug, PartialEq, Hash, Eq, Copy, Clone, Serialize, Deserialize)]
pub enum RecordHeader {
    /// A one byte record that, if read, result in a 0 checksum.
    Checksum, //always size 1
    /// A 0-0xFFFF record with nothing but zeros, usually lining the next
    /// record address.
    Filler(u16),
    /// Only two variations are known, a HWID and a PartNumber. Possibly
    /// describing the file format itself.
    MainHeader(u16),
    /// Arbitrary data, usually containing valid ASCII text, mostly one line.
    Text(u16),
    /// Data information about the firmware, contains only the type.
    ///
    /// This will be later combine with DescriptorData
    DescriptorType(u16),
    /// Data information about the firmware, contain only the data.
    ///
    /// Need to be combined with DescriptorType to be interpreted.
    DescriptorData(u16),
    /// Mark the end of the file. Can only be the last header.
    End, //always size 0
    /// Header with Unknown ID, can be a Firmware block, if the ID is described
    /// on the Descriptor header, or Unknown/Undefined Record.
    Unknown { id: u16, len: u16 },
}

impl RecordHeader {
    /// Return the id from the Header
    pub const fn id(&self) -> u16 {
        match self {
            RecordHeader::Unknown { id, .. } => *id,
            RecordHeader::Checksum => checksum::ID,
            RecordHeader::Filler(_) => filler::ID,
            RecordHeader::MainHeader(_) => main::ID,
            RecordHeader::Text(_) => text::ID,
            RecordHeader::DescriptorType(_) => descriptor::descriptor_type::ID,
            RecordHeader::DescriptorData(_) => descriptor::descriptor_data::ID,
            RecordHeader::End => 0xffff,
        }
    }
    /// Return the len from the Header, obs: not the len of the Header itself
    pub const fn len(&self) -> u16 {
        match self {
            RecordHeader::Unknown { len, .. } => *len,
            RecordHeader::Checksum => 0x0001,
            RecordHeader::Filler(len) => *len,
            RecordHeader::MainHeader(len) => *len,
            RecordHeader::Text(len) => *len,
            RecordHeader::DescriptorType(len) => *len,
            RecordHeader::DescriptorData(len) => *len,
            RecordHeader::End => 0,
        }
    }
    /// Create a header using the id and len values.
    pub const fn from_value(id: u16, len: u16) -> Self {
        match id {
            checksum::ID if len == 1 => RecordHeader::Checksum,
            filler::ID => RecordHeader::Filler(len),
            main::ID => RecordHeader::MainHeader(len),
            text::ID => RecordHeader::Text(len),
            descriptor::descriptor_type::ID => {
                RecordHeader::DescriptorType(len)
            }
            descriptor::descriptor_data::ID => {
                RecordHeader::DescriptorData(len)
            }
            0xFFFF if len == 0 => RecordHeader::End,
            _ => RecordHeader::Unknown { id, len },
        }
    }
    /// Create the Header using raw bytes
    pub fn from_raw<B: ByteOrder>(data: &[u8]) -> Result<(&[u8], Self)> {
        if data.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Record hreader buffer too small",
            ));
        }
        let id = B::read_u16(&data[..2]);
        let len = B::read_u16(&data[2..]);
        Ok((&data[4..], RecordHeader::from_value(id, len)))
    }
    /// Write the Header to the raw byte buffer.
    pub fn to_raw<'a, B: ByteOrder>(
        &self,
        data: &'a mut [u8],
    ) -> Result<&'a mut [u8]> {
        if data.len() < 4 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Record hreader buffer too small",
            ));
        }
        B::write_u16(data, self.id());
        B::write_u16(&mut data[2..], self.len());
        Ok(&mut data[4..])
    }
}

/// All known Records.
#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
pub enum Record {
    Checksum(ChecksumRecord),
    Filler(FillerRecord),
    MainHeader(MainRecord),
    Text(TextRecord),
    Descriptor(DescriptorRecord),
    FirmwareData(FirmwareRecord),
    End,
}

impl Display for Record {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Record::Checksum(x) => write!(f, "{}", x),
            Record::Filler(x) => write!(f, "{}", x),
            Record::MainHeader(x) => write!(f, "{}", x),
            Record::Text(x) => write!(f, "{}", x),
            Record::Descriptor(x) => write!(f, "{}", x),
            Record::FirmwareData(x) => write!(f, "{}", x),
            Record::End => write!(f, "Record:End"),
        }
    }
}
