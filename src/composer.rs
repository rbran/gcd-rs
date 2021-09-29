//! Compose new GCD file

use crate::record::checksum::{self, ChecksumRecord};
use crate::record::descriptor::DescriptorRecord;
use crate::record::filler::FillerRecord;
use crate::record::firmware::FirmwareRecord;
use crate::record::text::TextRecord;
use crate::{
    GcdDefaultEndian, MainRecord, Record, RecordHeader, RECORD_HEADER_LEN,
};
use byteorder::ByteOrder;
use std::io::{Result, Write};
use std::marker::PhantomData;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct WriteCheckSum<F> {
    file: F,
    sum: u8,
}
impl<F> Write for WriteCheckSum<F>
where
    F: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let len = self.file.write(buf)?;
        for byte in buf.iter() {
            self.sum = self.sum.wrapping_add(*byte);
        }
        Ok(len)
    }

    fn flush(&mut self) -> Result<()> {
        self.file.flush()
    }
}
impl<F> WriteCheckSum<F>
where
    F: std::io::Write,
{
    fn new(file: F) -> Self {
        WriteCheckSum { file, sum: 0 }
    }
}

impl<F> WriteCheckSum<F> {
    const fn sum(&self) -> u8 {
        self.sum
    }
}

pub struct Composer<F, B = GcdDefaultEndian>
where
    F: std::io::Write,
    B: ByteOrder,
{
    file: WriteCheckSum<F>,
    endian: PhantomData<B>,
}

impl<F, B> Composer<F, B>
where
    F: std::io::Write,
    B: ByteOrder,
{
    pub fn new(file: F) -> Result<Self> {
        //write signature and version (100)
        let mut sign = [0; 8];
        let mut file = WriteCheckSum::new(file);
        sign[..6].copy_from_slice(b"GARMIN");
        B::write_u16(&mut sign[6..], 100);
        file.write_all(&sign)?;
        Ok(Composer {
            file,
            endian: PhantomData,
        })
    }

    /// Write a record composed without any encoding
    pub fn write_record_raw(&mut self, id: u16, data: &[u8]) -> Result<()> {
        self.write_record_header(RecordHeader::Unknown {
            id,
            len: data.len() as u16,
        })?;
        self.file.write_all(&data)
    }
    /// Write a record, encoding its data
    pub fn write_record(&mut self, record: &Record) -> Result<()> {
        match record {
            Record::Checksum(_) => self.write_check_point(),
            Record::Filler(filler) => self.write_filler(filler),
            Record::MainHeader(header) => self.write_main(header),
            Record::Text(cop) => self.write_text(cop),
            Record::Descriptor(desc) => self.write_descriptor(desc),
            Record::FirmwareData(firm) => self.write_firmware(firm),
            Record::End => self.write_end(),
        }
    }

    fn write_record_header(&mut self, header: RecordHeader) -> Result<()> {
        let mut data = [0; 4];
        B::write_u16(&mut data[..2], header.id());
        B::write_u16(&mut data[2..], header.len());
        self.file.write_all(&data)
    }
    fn write_end(&mut self) -> Result<()> {
        self.write_record_header(RecordHeader::End)
    }
    fn write_firmware(&mut self, record: &FirmwareRecord) -> Result<()> {
        let mut data = vec![0; record.len() as usize + RECORD_HEADER_LEN];
        record.record_to_raw::<B>(&mut data)?;
        self.file.write_all(&data)
    }
    fn write_check_point(&mut self) -> Result<()> {
        let mut data = [0; checksum::LEN as usize + RECORD_HEADER_LEN];
        ChecksumRecord::record_to_raw::<B>(&mut data, self.file.sum())?;
        self.file.write_all(&data)
    }
    fn write_filler(&mut self, filler: &FillerRecord) -> Result<()> {
        let mut data = vec![0; filler.len() as usize + RECORD_HEADER_LEN];
        filler.record_to_raw::<B>(&mut data)?;
        self.file.write_all(&data)
    }
    fn write_main(&mut self, main: &MainRecord) -> Result<()> {
        let mut data = vec![0; main.len() as usize + RECORD_HEADER_LEN];
        main.record_to_raw::<B>(&mut data)?;
        self.file.write_all(&data)
    }
    fn write_text(&mut self, text: &TextRecord) -> Result<()> {
        let mut data = vec![0; text.len() as usize + RECORD_HEADER_LEN];
        text.record_to_raw::<B>(&mut data)?;
        self.file.write_all(&data)
    }
    fn write_descriptor<'a>(
        &mut self,
        descriptor: &DescriptorRecord,
    ) -> Result<()> {
        let desc_type_len = descriptor.record_type_len() as usize;
        let desc_data_len = descriptor.record_data_len() as usize;
        let mut data =
            vec![0; desc_type_len + desc_data_len + (RECORD_HEADER_LEN * 2)];

        let data_current = descriptor.record_type_to_raw::<B>(&mut data)?;
        descriptor.record_data_to_raw::<B>(data_current)?;
        self.file.write_all(&data)
    }
}

#[cfg(test)]
mod tests {
    //use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
    use crate::composer::{Composer, WriteCheckSum};
    use crate::record::descriptor::descriptor_data;
    use crate::record::descriptor::descriptor_data::DescriptorData;
    use crate::record::descriptor::descriptor_type;
    use crate::record::descriptor::DescriptorRecord;
    use crate::record::filler::FillerRecord;
    use crate::record::main::{self, MainRecord};
    use crate::record::text::TextRecord;
    use byteorder::{ByteOrder, BE, LE};
    use std::io::{Cursor, Result, Write};

    fn composer<B: ByteOrder>() -> Result<Composer<Cursor<Vec<u8>>, B>> {
        let file = Cursor::new(Vec::new());
        Composer::new(file)
    }

    fn extend_u16<B: ByteOrder>(vec: &mut Vec<u8>, x: u16) {
        let mut buf = [0u8; 2];
        B::write_u16(&mut buf, x);
        vec.extend(buf.iter())
    }

    #[test]
    fn check_sum() {
        let mut file = WriteCheckSum::new(Cursor::new(vec![0u8; 11]));
        file.write_all(&[0x1]).unwrap();
        file.write_all(&[0x2]).unwrap();
        file.write_all(&[0x3, 0x4]).unwrap();
        file.write_all(&[0x1, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1])
            .unwrap();
        assert_eq!(file.sum().wrapping_neg(), 244);
    }

    fn check_main<B: ByteOrder>(header: &MainRecord, data: &[u8]) {
        let mut composer = composer::<B>().unwrap();
        composer.write_main(header).unwrap();

        let mut result = vec![b'G', b'A', b'R', b'M', b'I', b'N'];
        extend_u16::<B>(&mut result, 100); //header version
        extend_u16::<B>(&mut result, 0x03);
        extend_u16::<B>(&mut result, data.len() as u16); //record len
        result.extend(data.iter());
        assert_eq!(composer.file.file.get_ref(), &result);
    }

    #[test]
    fn write_main() {
        let main_header_hwid = MainRecord::DefaultHWID;
        let main_header_pn = MainRecord::DefaultPartNumber;

        let mut default_hwid_le = [0; 2];
        let mut default_hwid_be = [0; 2];
        LE::write_u16(&mut default_hwid_le, main::DEFAULT_HWID);
        BE::write_u16(&mut default_hwid_be, main::DEFAULT_HWID);
        check_main::<LE>(&main_header_hwid, &default_hwid_le);
        check_main::<BE>(&main_header_hwid, &default_hwid_be);

        let mut default_pn_le = [0; 9];
        let mut default_pn_be = [0; 9];
        LE::write_uint128(&mut default_pn_le, main::DEFAULT_PART_NUMBER, 9);
        BE::write_uint128(&mut default_pn_be, main::DEFAULT_PART_NUMBER, 9);
        check_main::<LE>(&main_header_pn, &default_pn_le);
        check_main::<BE>(&main_header_pn, &default_pn_be);
    }

    fn check_text<B: ByteOrder>(text: &TextRecord) {
        let mut composer = composer::<B>().unwrap();
        composer.write_text(text).unwrap();
        composer.write_end().unwrap();

        let mut result = vec![b'G', b'A', b'R', b'M', b'I', b'N'];
        extend_u16::<B>(&mut result, 100); //header version
        extend_u16::<B>(&mut result, 0x05); //record id
        extend_u16::<B>(&mut result, text.len() as u16); //record len
        result.extend(text.value());
        extend_u16::<B>(&mut result, 0xffff); //record end id
        extend_u16::<B>(&mut result, 0x0000); //record end len
        assert_eq!(composer.file.file.get_ref(), &result);
    }

    #[test]
    fn write_text() {
        let text = TextRecord::Simple("The text is XXXXXXXXX".to_string());
        check_text::<LE>(&text);
        check_text::<BE>(&text);
        let text = TextRecord::Blob(b"The text is YYYYYYYYY".to_vec());
        check_text::<LE>(&text);
        check_text::<BE>(&text);
    }

    fn check_filler<B: ByteOrder>(len: u16) {
        let mut composer = composer::<B>().unwrap();
        let filler = FillerRecord::Zeros(len);
        composer.write_filler(&filler).unwrap();

        let mut result = vec![b'G', b'A', b'R', b'M', b'I', b'N'];
        extend_u16::<B>(&mut result, 100); //header version
        extend_u16::<B>(&mut result, 0x02); //record id
        extend_u16::<B>(&mut result, len); //record len
        result.extend(vec![0u8; len as usize].iter());
        assert_eq!(composer.file.file.get_ref(), &result);
    }

    #[test]
    fn write_filler() {
        check_filler::<LE>(100);
        check_filler::<BE>(100);
    }

    fn check_checkpoint<B: ByteOrder>() {
        let mut composer = composer::<B>().unwrap();
        composer.write_check_point().unwrap();

        let mut result = vec![b'G', b'A', b'R', b'M', b'I', b'N'];
        extend_u16::<B>(&mut result, 100); //header version
        extend_u16::<B>(&mut result, 0x01); //record id
        extend_u16::<B>(&mut result, 1); //record len
        result.push(0xDC); //checksum value
        assert_eq!(composer.file.file.get_ref(), &result);
    }

    #[test]
    fn write_checksum() {
        check_checkpoint::<LE>();
        check_checkpoint::<BE>();
    }

    fn check_descriptor<B: ByteOrder>(desc: &DescriptorRecord) {
        // generate using the compositor
        let mut composer = composer::<B>().unwrap();
        composer.write_descriptor(desc).unwrap();

        // generate manualy
        let desc_type_len = desc.record_type_len();
        let desc_data_len = desc.record_data_len();
        let mut result = vec![0;
            8 + //sig
            4 + // desc type header
            desc_type_len as usize + // desc type body
            4 + // desc data header
            desc_data_len as usize //desc data body
        ];

        {
            //manually generate the file on the result vec
            //file signature
            let mut result_current = result.as_mut_slice();
            result_current[..6].copy_from_slice(b"GARMIN");
            result_current = &mut result_current[6..];
            B::write_u16(result_current, 100); //header version
            result_current = &mut result_current[2..];

            //write descriptor type record
            B::write_u16(result_current, descriptor_type::ID); //record id
            result_current = &mut result_current[2..];
            B::write_u16(result_current, desc_type_len); //record len
            result_current = &mut result_current[2..];
            for descriptor in desc.iter() {
                result_current = descriptor
                    .descriptor_type()
                    .to_raw::<B>(result_current)
                    .unwrap();
            }

            //write decriptor data record
            B::write_u16(result_current, descriptor_data::ID); //record id
            result_current = &mut result_current[2..];
            B::write_u16(result_current, desc_data_len); //record len
            result_current = &mut result_current[2..];
            for descriptor in desc.iter() {
                result_current =
                    descriptor.to_raw::<B>(result_current).unwrap();
            }
        }

        assert_eq!(composer.file.file.get_ref().as_slice(), result.as_slice());
    }

    #[test]
    fn write_descriptor() {
        let descriptor = DescriptorRecord::Simple(vec![
            DescriptorData::U8 { id: 1, data: 0 },
            DescriptorData::U16 { id: 1, data: 0 },
            DescriptorData::U32 { id: 1, data: 0 },
            DescriptorData::U64 { id: 1, data: 0 },
            DescriptorData::Other {
                id: 1,
                data: vec![0, 1, 2],
            },
            DescriptorData::End,
        ]);
        check_descriptor::<LE>(&descriptor.clone());
        check_descriptor::<BE>(&descriptor);
    }
}
