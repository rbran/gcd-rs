//! Parse an existing GCD file.

use byteorder::ByteOrder;
use std::io::{Error, ErrorKind, Read, Result};

use crate::record::checksum::ChecksumRecord;
use crate::record::descriptor::descriptor_data::DescriptorDecoded;
use crate::record::descriptor::{DescriptorRecord, DescriptorTypeRecord};
use crate::record::filler::FillerRecord;
use crate::record::firmware::FirmwareRecord;
use crate::record::main::MainRecord;
use crate::record::text::TextRecord;
use crate::{GcdDefaultEndian, Record, RecordHeader};

use std::marker::PhantomData;

//Parser state, acusing if data is out of order in the file
// T  => TextRecord
// M  => MainRecord
// DT => DescriptorTypeRecord
// DD => DescriptorDataRecord
// FD => FirmwareDataRecord
// E  => EndRecord
//
// File: C* M C* (DT DD FD* C*)+ E
#[derive(Debug, PartialEq, Copy, Clone)]
enum ParseState {
    TextGlobal,
    Main,
    DescriptorType,
    DescriptorData,
    FirmwareData,
    End,
}

struct ReadCheckSum<F> {
    file: F,
    sum: u8,
}

impl<F> Read for ReadCheckSum<F>
where
    F: std::io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.file.read(buf)?;
        for byte in buf[0..read].iter() {
            self.sum = self.sum.wrapping_add(*byte);
        }
        Ok(read)
    }
}

impl<F> ReadCheckSum<F>
where
    F: std::io::Read,
{
    fn new(file: F) -> Self {
        ReadCheckSum { file, sum: 0 }
    }
}

impl<F> ReadCheckSum<F> {
    const fn sum(&self) -> u8 {
        self.sum
    }
}

// information extracted from Descriptor used to process the firmware chunk
#[derive(Default)]
struct FirmwareData {
    // id of the firmware record
    id: u16,
    // xor key used to decode the firmware, 0 is no key
    xor_key: u8,
    // firmware total lenght
    lenght: u32,
    // firmware len that need to be consumend before the end
    lenght_left: u32,
}

pub struct Parser<F, B = GcdDefaultEndian>
where
    F: std::io::Read,
    B: ByteOrder,
{
    state: ParseState,
    file: ReadCheckSum<F>,
    descriptor_type: DescriptorTypeRecord,
    firmware: FirmwareData,
    endian: PhantomData<B>,
}

impl<F, B> Parser<F, B>
where
    F: std::io::Read,
    B: ByteOrder,
{
    pub fn new(file: F) -> Result<Self> {
        let state = ParseState::TextGlobal;
        let mut file = ReadCheckSum::new(file);

        let mut header_sign = [0u8; 8];
        file.read_exact(&mut header_sign)?;
        match &header_sign[..6] {
            b"GARMIN" => {}
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid/Unknown Header Signature",
                ))
            }
        }

        let header_version = B::read_u16(&header_sign[6..]);
        match header_version {
            100 => {}
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Invalid/Unknown Header Version",
                ))
            }
        }

        Ok(Self {
            state,
            file,
            descriptor_type: Default::default(),
            firmware: Default::default(),
            endian: PhantomData,
        })
    }

    /// Read the next available record
    pub fn read_record(&mut self) -> Result<Record> {
        //loop until error or return a record
        loop {
            if let ParseState::End = self.state {
                //TODO check if there is more data after the End Record and return
                //Err if there is.
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Unable to read after End Record",
                ));
            }

            let state = self.state; //avoid multiple borrows
            let record = self.parse_record()?;

            //check if we are allowed to receive this record on the current state
            match (state, record) {
                //CheckPoint and Filler are allowed at any state
                (_, RecordHeader::Checksum) => {
                    //Check Point, verify the sum
                    return Ok(Record::Checksum(self.parse_checksum()?));
                }
                (_, RecordHeader::Filler(len)) => {
                    return Ok(Record::Filler(self.parse_filler(len)?))
                }

                //Didn't Received the MainHeader yet
                (ParseState::TextGlobal, RecordHeader::Text(len)) => {
                    return Ok(Record::Text(self.parse_text(len)?));
                }
                (ParseState::TextGlobal, RecordHeader::MainHeader(len)) => {
                    //Main Header, change state so we refuse a second one
                    self.state = ParseState::Main;
                    return Ok(Record::MainHeader(
                        self.parse_main_header(len)?,
                    ));
                }

                //Received MainHeader
                (ParseState::Main, RecordHeader::DescriptorType(len)) => {
                    //first firmware block, no more global data
                    self.state = ParseState::DescriptorType;
                    //at this state descriptor_type is sure to be NONE
                    self.descriptor_type = self.parse_descriptor_type(len)?;
                }
                (ParseState::Main, RecordHeader::Text(len)) => {
                    // Text(after Main Header)
                    return Ok(Record::Text(self.parse_text(len)?));
                }

                //Received the firmware descriptor type
                (
                    ParseState::DescriptorType,
                    RecordHeader::DescriptorData(len),
                ) => {
                    self.state = ParseState::DescriptorData;
                    //at this state is garantied that descriptor_type is Some()
                    return Ok(Record::Descriptor(
                        self.parse_descriptor_data(len)?,
                    ));
                }

                //received the firmware descriptor type and data
                (
                    ParseState::DescriptorData,
                    RecordHeader::DescriptorType(len),
                ) => {
                    //received a new firmware, Firmware Data Record missing
                    self.state = ParseState::DescriptorType;
                    //TODO: allow Firmware Data Record missing?
                    //is garantied that self.firmware in Some at this state
                    self.check_firmware_end()?;
                    self.descriptor_type = self.parse_descriptor_type(len)?;
                }
                (
                    ParseState::DescriptorData,
                    RecordHeader::Unknown { id, len },
                ) => {
                    //first data chunk received
                    self.state = ParseState::FirmwareData;
                    //send this data chunk
                    return Ok(Record::FirmwareData(
                        self.parse_firmware_data(id, len)?,
                    ));
                }
                (ParseState::DescriptorData, RecordHeader::Text(len)) => {
                    //firmware text, no firmware data received yet
                    return Ok(Record::Text(self.parse_text(len)?));
                }
                (ParseState::DescriptorData, RecordHeader::End) => {
                    //firmware block only had descriptor
                    //current block don't have data or text
                    self.state = ParseState::End;
                    //end this firmware
                    self.check_firmware_end()?;
                    return Ok(Record::End);
                }

                // text or firmware data
                (ParseState::FirmwareData, RecordHeader::Text(len)) => {
                    let text = self.parse_text(len)?;
                    return Ok(Record::Text(text));
                }
                (
                    ParseState::FirmwareData,
                    RecordHeader::Unknown { id, len },
                ) => {
                    //second or more data chunk received
                    //send this data chunk
                    return Ok(Record::FirmwareData(
                        self.parse_firmware_data(id, len)?,
                    ));
                }
                (ParseState::FirmwareData, RecordHeader::End) => {
                    //not more Firmware Data
                    self.state = ParseState::End;
                    //end this firmware
                    self.check_firmware_end()?;
                    return Ok(Record::End);
                }
                (
                    ParseState::FirmwareData,
                    RecordHeader::DescriptorType(len),
                ) => {
                    //received a new firmware after receiving a firmware
                    //block, with at least text
                    self.state = ParseState::DescriptorType;
                    //end this firmware
                    self.check_firmware_end()?;
                    self.parse_descriptor_type(len)?;
                }

                (state, record) => {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "State {:?} record received {:?}",
                            state, record
                        ),
                    ));
                }
            }
        }
    }

    fn parse_record(&mut self) -> Result<RecordHeader> {
        let mut header = [0; 4];
        self.file.read_exact(&mut header)?;
        let (_, ret) = RecordHeader::from_raw::<B>(&mut header)?;
        Ok(ret)
    }

    fn parse_checksum(&mut self) -> Result<ChecksumRecord> {
        let mut data = [0];
        self.file.read_exact(&mut data)?;
        let checksum = self.file.sum();
        return Ok(ChecksumRecord::new(&data, checksum)?);
    }

    fn parse_filler(&mut self, lenght: u16) -> Result<FillerRecord> {
        let mut data = vec![0; lenght as usize];
        self.file.read_exact(&mut data)?;
        return Ok(FillerRecord::new(&data)?);
    }

    fn parse_main_header(&mut self, lenght: u16) -> Result<MainRecord> {
        MainRecord::new::<ReadCheckSum<F>, B>(&mut self.file, lenght)
    }

    fn parse_text(&mut self, lenght: u16) -> Result<TextRecord> {
        TextRecord::new(&mut self.file, lenght)
    }

    fn parse_descriptor_type(
        &mut self,
        lenght: u16,
    ) -> Result<DescriptorTypeRecord> {
        DescriptorTypeRecord::new::<ReadCheckSum<F>, B>(&mut self.file, lenght)
    }

    fn parse_descriptor_data(&mut self, lenght: u16) -> Result<DescriptorRecord>
    where
        F: std::io::Read,
    {
        let descriptor = DescriptorRecord::new::<ReadCheckSum<F>, B>(
            &mut self.file,
            lenght,
            &self.descriptor_type,
        )?;

        //find extract necessary data
        let mut firmware_id = None;
        let mut firmware_lenght = None;
        let mut xor_key = None;
        //TODO make a more elegant data extraction
        for desc in descriptor.iter() {
            match desc.decode() {
                Some(DescriptorDecoded::FirmwareId(x)) => {
                    firmware_id = Some(x);
                }
                Some(DescriptorDecoded::FirmwareLen(x)) => {
                    firmware_lenght = Some(x);
                }
                Some(DescriptorDecoded::XorKey(x)) => {
                    xor_key = Some(x);
                }
                Some(DescriptorDecoded::Firmware2000P1Len(x))
                | Some(DescriptorDecoded::Firmware2000P2Len(x))
                | Some(DescriptorDecoded::Firmware2000P3Len(x)) => {
                    //TODO is the size for each part?
                    //each part is separated? in sequence?
                    firmware_lenght = Some(x);
                }
                Some(_) => {}
                None => {}
            }
        }
        //TODO check if those values exist on Firmware Descriptor Type parsing
        match firmware_id {
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Firmware Id not found",
                ))
            }
            Some(x) => self.firmware.id = x,
        }
        match firmware_lenght {
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Firmware Lenght not found",
                ))
            }
            Some(x) => self.firmware.lenght = x,
        }
        self.firmware.xor_key = xor_key.unwrap_or(0);
        self.firmware.lenght_left = self.firmware.lenght;
        Ok(descriptor)
    }

    fn parse_firmware_data(
        &mut self,
        record_id: u16,
        record_len: u16,
    ) -> Result<FirmwareRecord>
    where
        F: std::io::Read,
    {
        if record_id != self.firmware.id {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Firmware id expected {:#x} found {:#x}",
                    self.firmware.id, record_id,
                ),
            ));
        }
        //subtract the current consumed firmware chunk
        if self.firmware.lenght_left < record_len as u32 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Firmware Chunk is bigger than expected",
            ));
        }
        self.firmware.lenght_left -= record_len as u32;
        //send chunk to handle
        let mut buf = vec![0u8; record_len as usize];
        self.file.read_exact(&mut buf)?;
        if self.firmware.xor_key != 0 {
            buf.iter_mut().for_each(|x| *x = *x ^ self.firmware.xor_key);
        }
        match self.firmware.id {
            // TrueType font file, XORed with 0x76
            0x05A5 => buf.iter_mut().for_each(|x| *x = *x ^ 0x76),
            _ => {}
        }
        Ok(FirmwareRecord::new(buf, record_id))
    }

    fn check_firmware_end(&mut self) -> Result<()> {
        //check if the firmware was fully received
        if self.firmware.lenght_left != 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Firmware Chunk too small, received {} from {} bytes",
                    self.firmware.lenght - self.firmware.lenght_left,
                    self.firmware.lenght
                ),
            ));
        }
        Ok(())
    }
}
