# GCD Prarser and Composer

This lib helps read/write GCD files.

## Reading GCD Files

```rust
use std::env;
use std::fs::File;
use gcd_rs::parser::Parser;
use gcd_rs::Record;

fn main() {
    //open the gcd file
    let file = File::open("in_file.gcd").unwrap();

    //parser
    let mut parser: Parser<File> = Parser::new(file).unwrap();

    loop {
        //read and print the record until the End is received
        let record = parser.read_record().expect("Unable to read record");
        println!("Record {}", record);
        if let Record::End = record {
            break;
        }
    }
}
```

## Writing GCD File
```rust
use gcd_rs::composer::Composer;
use gcd_rs::record::text::TextRecord;
use gcd_rs::Record;
use std::env;
use std::fs::File;

fn main() {
    //create the gcd file
    let file = File::create("out_file.gcd").unwrap();

    //composer
    let mut composer: Composer<File> = Composer::new(file).unwrap();

    //write a text record
    composer
        .write_record(&Record::Text(TextRecord::Simple(
            "Sample File".to_string(),
        )))
        .unwrap();
    //write the end record
    composer.write_record(&Record::End).unwrap();
}

```

