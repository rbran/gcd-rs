use gcd_rs::parser::Parser;
use gcd_rs::record::descriptor::descriptor_data::DescriptorDecoded;
use gcd_rs::Record;

use std::env;
use std::fs::File;
use std::io::Write;

mod serialize;
use serialize::{ExtFirmware, RecordSerialized};

use serde_yaml;

struct FirmwareFile {
    file: File,
    ext_firmware: ExtFirmware,
}

// This open the gcd file and create a simple serialized version (toml) of it,
// except for the firmware data, that is stored in separated files.
fn main() {
    //filenames from args
    let args = env::args().collect::<Vec<String>>();
    let filename = args.get(1).unwrap();
    let filename_out = args.get(2).unwrap();

    //open the gcd file
    let file = File::open(filename).unwrap();

    //parser
    let mut parser: Parser<File> = Parser::new(file).unwrap();

    // struct to serialize
    let mut records: Vec<RecordSerialized> = vec![];

    //external file used to write the firmware Data
    let mut firmware_out = None;
    // Some files have multiple firmware with the same id, so also have a
    // counter to create a unique filename
    let mut fw_num = 0;

    loop {
        // translate the enum Record into RecordSerialized
        match parser.read_record().expect("Unable to read record") {
            // create a new firmware file
            Record::Descriptor(descriptors) => {
                //get the firmware id
                let id = descriptors
                    .iter()
                    .find_map(|x| {
                        if let Some(DescriptorDecoded::FirmwareId(x)) =
                            x.decode()
                        {
                            Some(x)
                        } else {
                            None
                        }
                    })
                    .expect("Unable to find firmware ID");

                //create the file
                let filename = format!("fw{}_0x{}.bin", fw_num, id);
                let file = File::create(filename.clone()).unwrap();
                let firmware = ExtFirmware {
                    filename,
                    id,
                    offset: 0,
                    lenght: 0,
                };

                //this also close the last file, if it exists
                firmware_out = Some(FirmwareFile {
                    file,
                    ext_firmware: firmware,
                });

                fw_num += 1;
                records.push(Record::Descriptor(descriptors).into());
            }
            // write the firmware data and repace Record::FirmwareData, with
            // RecordSerialized::External(Firmware)
            Record::FirmwareData(fw_record) => {
                let firmware = firmware_out.as_mut().unwrap();
                //write the chunk of data on the fw file
                firmware.file.write_all(fw_record.data()).unwrap();
                //set and push the external firmware record
                firmware.ext_firmware.lenght = fw_record.len() as u64;
                records.push(RecordSerialized::External(
                    firmware.ext_firmware.clone(),
                ));
                //advance the offset for the next chunk
                firmware.ext_firmware.offset += fw_record.len() as u64;
            }
            // End of Gcd File
            record @ Record::End => {
                records.push(record.into());
                break;
            }
            record => records.push(record.into()),
        }
    }
    //write the serialized file
    let file_out = File::create(filename_out).unwrap();
    serde_yaml::to_writer(file_out, &records).unwrap();
}
