// Edge case: Floating point types

use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct FloatingPoint {
    pub small: f32,
    pub large: f64,
    pub vec_f32: Vec<f32>,
    pub vec_f64: Vec<f64>,
    pub opt_f32: Option<f32>,
    pub array_f64: [f64; 4],
}

fn main() {}
