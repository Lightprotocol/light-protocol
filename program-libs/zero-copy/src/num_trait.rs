use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

pub trait ZeroCopyNumTrait:
    Add
    + Sub
    + AddAssign
    + SubAssign
    + Div
    + DivAssign
    + Mul
    + MulAssign
    + std::marker::Sized
    + From<u64>
    + Into<u64>
    + Copy
    + std::convert::TryFrom<u64>
{
    fn to_bytes_le(&self) -> [u8; 8];
    fn to_bytes_be(&self) -> [u8; 8];
}

impl ZeroCopyNumTrait for u64 {
    fn to_bytes_le(&self) -> [u8; 8] {
        self.to_le_bytes()
    }
    fn to_bytes_be(&self) -> [u8; 8] {
        self.to_be_bytes()
    }
}

impl ZeroCopyNumTrait for zerocopy::little_endian::U64 {
    fn to_bytes_le(&self) -> [u8; 8] {
        self.to_bytes()
    }
    fn to_bytes_be(&self) -> [u8; 8] {
        let mut bytes = self.to_bytes();
        bytes.reverse();
        bytes
    }
}
