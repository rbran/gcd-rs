use gcd::composer::Composer;
use gcd::record::text::TextRecord;
use gcd::Record;
use std::env;
use std::fs::File;

fn main() {
    //filenames from args
    let args = env::args().collect::<Vec<String>>();
    let filename = args.get(1).unwrap();

    //open the gcd file
    let file = File::create(filename).unwrap();

    //composer
    let mut composer: Composer<File> = Composer::new(file).unwrap();

    composer
        .write_record(&Record::Text(TextRecord::Simple(
            "Sample File".to_string(),
        )))
        .unwrap();
    composer.write_record(&Record::End).unwrap();
}
