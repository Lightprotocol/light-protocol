// Edge case: Combination of all features
#![cfg(feature="mut")] 
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

// Import Pubkey from the test helper
#[path = "../../instruction_data.rs"]
mod instruction_data;
use instruction_data::Pubkey;

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct CombinationAllFeatures {
    // Meta fields
    pub meta1: u32,
    pub meta2: bool,
    pub meta3: [u8; 4],
    
    // Dynamic fields
    pub vec1: Vec<u8>,
    pub opt1: Option<u64>,
    pub vec2: Vec<u32>,
    pub opt2: Option<u32>,
    
    // Special types
    pub pubkey: Pubkey,
    
    // More dynamic
    pub vec3: Vec<bool>,
    pub opt3: Option<Vec<u8>>,
    
    // Arrays after dynamic
    pub arr1: [u64; 16],
    
    // Final primitives
    pub final1: u32,
    pub final2: bool,
}

fn main() {}