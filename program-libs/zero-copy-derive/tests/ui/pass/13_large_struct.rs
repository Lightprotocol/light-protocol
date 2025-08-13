// Edge case: Very large struct (25+ fields)
#![cfg(feature="mut")] use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct LargeStruct {
    pub field_001: u32,
    pub field_002: u32,
    pub field_003: u32,
    pub field_004: u32,
    pub field_005: u32,
    pub field_006: u32,
    pub field_007: u32,
    pub field_008: u32,
    pub field_009: u32,
    pub field_010: u32,
    pub field_011: u32,
    pub field_012: u32,
    pub field_013: u32,
    pub field_014: u32,
    pub field_015: u32,
    pub field_016: u32,
    pub field_017: u32,
    pub field_018: u32,
    pub field_019: u32,
    pub field_020: u32,
    pub field_021: Vec<u8>,
    pub field_022: Option<u64>,
    pub field_023: u32,
    pub field_024: u32,
    pub field_025: u32,
}

fn main() {}