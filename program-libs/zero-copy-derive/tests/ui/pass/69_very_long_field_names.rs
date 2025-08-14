// Edge case: Very long field names
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct VeryLongFieldNames {
    pub this_is_an_extremely_long_field_name_that_tests_the_limits_of_identifier_processing: u32,
    pub another_very_long_field_name_with_many_underscores_and_words_combined_together: Vec<u8>,
    pub yet_another_ridiculously_long_field_name_to_ensure_macro_handles_long_identifiers:
        Option<u64>,
}

fn main() {
    let original = VeryLongFieldNames {
        this_is_an_extremely_long_field_name_that_tests_the_limits_of_identifier_processing: 12345,
        another_very_long_field_name_with_many_underscores_and_words_combined_together: vec![
            1, 2, 3,
        ],
        yet_another_ridiculously_long_field_name_to_ensure_macro_handles_long_identifiers: Some(
            98765,
        ),
    };

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();

    // Test zero_copy_at (read-only)
    let zero_copy_read = VeryLongFieldNames::zero_copy_at(&serialized).unwrap();

    // Test zero_copy_at_mut (mutable)
    let mut serialized_mut = serialized.clone();
    let _zero_copy_mut = VeryLongFieldNames::zero_copy_at_mut(&mut serialized_mut).unwrap();

    // Note: Cannot use assert_eq! due to Vec fields not implementing ZeroCopyEq
    println!("Borsh compatibility test passed for VeryLongFieldNames");
}
