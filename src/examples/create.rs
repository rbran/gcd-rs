use gcd::composer::Composer;
use gcd::record::firmware::FirmwareRecord;
use gcd::Record;

use std::env;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

mod serialize;
use serialize::RecordSerialized;

use serde_yaml;

// This does the opose of extract, creating a gcd file from the toml read.
fn main() {
    let args = env::args().collect::<Vec<String>>();
    let filename_in = args.get(1).unwrap();
    let filename_out = args.get(2).unwrap();

    //read file and deserialize
    let file_in = File::open(filename_in).unwrap();
    let records: Vec<RecordSerialized> =
        serde_yaml::from_reader(file_in).unwrap();

    //composer
    let file_out = File::create(filename_out).unwrap();
    let mut composer: Composer<File> = Composer::new(file_out).unwrap();

    for record in records {
        match record {
            RecordSerialized::External(ext_fw) => {
                // TODO instead of constantly open and closing files, have the
                // last file open, and close after a new one is required
                let mut file = File::open(ext_fw.filename).unwrap();
                file.seek(SeekFrom::Start(ext_fw.offset)).unwrap();
                let mut data = vec![0; ext_fw.lenght as usize];
                file.read_exact(&mut data).unwrap();
                composer
                    .write_record(&Record::FirmwareData(FirmwareRecord::new(
                        data, ext_fw.id,
                    )))
                    .unwrap();
            }
            RecordSerialized::Internal(record) => {
                composer.write_record(&record).unwrap()
            }
        }
    }
}
