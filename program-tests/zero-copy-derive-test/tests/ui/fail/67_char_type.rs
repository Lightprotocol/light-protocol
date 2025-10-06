// Edge case: Char type fields

use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct CharFields {
    pub single_char: char,
    pub chars: Vec<char>,
    pub maybe_char: Option<char>,
    pub char_array: [char; 10],
}

fn main() {}
