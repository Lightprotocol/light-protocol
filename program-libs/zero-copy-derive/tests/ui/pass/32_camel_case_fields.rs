// Edge case: CamelCase field names
#![cfg(feature="mut")] 
#![allow(non_snake_case)]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct CamelCaseFields {
    pub MyField: u32,
    pub AnotherField: Vec<u8>,
    pub YetAnotherField: Option<u64>,
}

fn main() {}
