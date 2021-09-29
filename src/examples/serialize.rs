use gcd::Record;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ExtFirmware {
    pub filename: String,
    pub id: u16,
    pub offset: u64,
    pub lenght: u64,
}
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum RecordSerialized {
    Internal(Record),
    External(ExtFirmware),
}

impl From<Record> for RecordSerialized {
    fn from(x: Record) -> Self {
        RecordSerialized::Internal(x)
    }
}
