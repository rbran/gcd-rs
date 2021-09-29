//! Part Number could represent a product or part of one, maybe a file format.
//!
//! The string representation of Part Number is "AAA-BCCCC-DD"
//!
//! Is composed of at least 4 parts. Is not know the real signification of
//! each value, based on suposition they are possibly:
//! A: Product Kind
//! B: Hw Type
//! C: Hw Id
//! D: Release/Variation
//!
//! Is also possible the fields C-D vary with meaning based on the value of A.
//!
//! It is represented as [u8; 9], is basically a string but each char is
//! (including '-') calculated but subtracting 0x20 and is 6 bits.

use byteorder::ByteOrder;
use nom::bytes::complete::*;
use nom::character::is_digit;
use nom::combinator::map_res;
use nom::sequence::tuple;
use nom::IResult;
use serde::{Deserialize, Serialize};
use std::{
    io::{Error, ErrorKind, Result},
    str::FromStr,
};

/// The only know representation of PartNumber
#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
pub struct PnSimple {
    kind: u16,
    hw_kind: u8,
    hw_id: u16,
    rel: u8,
}

/// PartNumber could represent, software, device, or part of a device.
#[derive(Debug, PartialEq, Hash, Eq, Clone, Serialize, Deserialize)]
//TODO Simple is not good, I need to check more PNs.
pub enum PartNumber {
    /// The simple AAA-BCCCC-DD format
    Simple(PnSimple),
}
impl PartNumber {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        //parsers
        let sep = tag(b"-");
        let is_kind = take_while_m_n(3, 3, is_digit);
        let hw_kind = take(1usize);
        let is_hw_id = take_while_m_n(4, 4, is_digit);
        let hw_id = map_res(is_hw_id, |x: &[u8]| {
            u16::from_str(&String::from_utf8_lossy(x))
        });
        let is_rel = take_while_m_n(2, 2, is_digit);

        let (input, kind) = map_res(is_kind, |x: &[u8]| {
            u16::from_str(&String::from_utf8_lossy(x))
        })(input)?;
        let (input, _) = sep(input)?;
        let (input, (hw_kind, hw_id)) = tuple((hw_kind, hw_id))(input)?;
        let (input, _) = sep(input)?;
        let (input, rel) = map_res(is_rel, |x: &[u8]| {
            u8::from_str(&String::from_utf8_lossy(x))
        })(input)?;

        Ok((
            input,
            PartNumber::Simple(PnSimple {
                kind,
                hw_kind: hw_kind[0] - b'0',
                hw_id,
                rel,
            }),
        ))
    }

    pub fn from_raw<B: ByteOrder>(x: &[u8]) -> Result<(&[u8], PartNumber)> {
        if x.len() < 9 {
            Err(Error::new(
                ErrorKind::InvalidData,
                "Part number buffer too small",
            ))
        } else {
            const fn base6(x: u128, byte: u8) -> u8 {
                (((x & (0b111111 << (6 * byte))) >> (6 * byte)) & 0xffu128)
                    as u8
            }
            const fn get_value(x: u128) -> [u8; 12] {
                let (mut ret, mut i) = ([0; 12], 0);
                while i < 12 {
                    ret[i] = base6(x, 11 - i as u8).wrapping_add(0x20);
                    i += 1;
                }
                ret
            }
            let num = B::read_uint128(x, 9);
            let buff = get_value(num);
            let (_, ret) = PartNumber::parse(&buff).map_err(|_| {
                Error::new(ErrorKind::InvalidData, "Unable to parse PartNumber")
            })?;
            Ok((&x[9..], ret))
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        let bytes = s.as_bytes();
        if bytes.len() < 12 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "PartNumber Invalid size",
            ));
        }
        let (_, ret) = PartNumber::parse(s.as_bytes()).map_err(|_| {
            Error::new(ErrorKind::InvalidData, "Unable to parse PartNumber")
        })?;
        Ok(ret)
    }
}

impl ToString for PartNumber {
    fn to_string(&self) -> String {
        match self {
            PartNumber::Simple(PnSimple {
                kind,
                hw_kind,
                hw_id,
                rel,
            }) => format!("{:03}-{}{:04}-{:02}", kind, hw_kind, hw_id, rel),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::PartNumber;

    /// Check if Part number is decoding raw data correctly
    #[test]
    fn part_number_from_bytes() {
        let bytes_little: Vec<u8> =
            vec![0x10, 0xD4, 0x5C, 0x13, 0x04, 0x45, 0x0D, 0x14, 0x41];
        let bytes_big: Vec<u8> = bytes_little.iter().rev().copied().collect();

        let (_, pn_little) =
            PartNumber::from_raw::<byteorder::LE>(&bytes_little).unwrap();
        let (_, pn_big) =
            PartNumber::from_raw::<byteorder::BE>(&bytes_big).unwrap();

        for pn in [pn_little, pn_big].iter() {
            assert_eq!(pn.to_string(), "010-10037-00");
        }
    }

    /// Parse invalid text to partnumber
    #[test]
    #[should_panic]
    fn part_number_invalid_str1() {
        let text = "010-ç0037-00";
        PartNumber::from_str(text).unwrap();
    }
    /// Parse invalid text to partnumber
    #[test]
    #[should_panic]
    fn part_number_invalid_str2() {
        let text = "010-0037-00";
        PartNumber::from_str(text).unwrap();
    }
}
