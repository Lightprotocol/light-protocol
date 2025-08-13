// Edge case: Very long field names
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(ZeroCopy, ZeroCopyMut)]
#[repr(C)]
pub struct VeryLongFieldNames {
    pub this_is_an_extremely_long_field_name_that_tests_the_limits_of_identifier_processing: u32,
    pub another_very_long_field_name_with_many_underscores_and_words_combined_together: Vec<u8>,
    pub yet_another_ridiculously_long_field_name_to_ensure_macro_handles_long_identifiers:
        Option<u64>,
}

fn main() {}
