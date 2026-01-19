//! Unit tests for LightAccount-derived traits
//!
//! Tests individual traits derived by the `LightAccount` macro on account data structs.

#[path = "account_macros/shared.rs"]
pub mod shared;

#[path = "account_macros/d1_single_pubkey_test.rs"]
pub mod d1_single_pubkey_test;

#[path = "account_macros/d1_multi_pubkey_test.rs"]
pub mod d1_multi_pubkey_test;

#[path = "account_macros/d1_no_pubkey_test.rs"]
pub mod d1_no_pubkey_test;

#[path = "account_macros/d1_option_primitive_test.rs"]
pub mod d1_option_primitive_test;

#[path = "account_macros/d1_option_pubkey_test.rs"]
pub mod d1_option_pubkey_test;

#[path = "account_macros/d1_non_copy_test.rs"]
pub mod d1_non_copy_test;

#[path = "account_macros/d1_array_test.rs"]
pub mod d1_array_test;

#[path = "account_macros/d1_all_field_types_test.rs"]
pub mod d1_all_field_types_test;

#[path = "account_macros/d2_single_compress_as_test.rs"]
pub mod d2_single_compress_as_test;

#[path = "account_macros/d2_multiple_compress_as_test.rs"]
pub mod d2_multiple_compress_as_test;

#[path = "account_macros/d2_no_compress_as_test.rs"]
pub mod d2_no_compress_as_test;

#[path = "account_macros/d2_option_none_compress_as_test.rs"]
pub mod d2_option_none_compress_as_test;

#[path = "account_macros/d2_all_compress_as_test.rs"]
pub mod d2_all_compress_as_test;

#[path = "account_macros/d4_minimal_test.rs"]
pub mod d4_minimal_test;

#[path = "account_macros/d4_info_last_test.rs"]
pub mod d4_info_last_test;

#[path = "account_macros/d4_large_test.rs"]
pub mod d4_large_test;

#[path = "account_macros/d4_all_composition_test.rs"]
pub mod d4_all_composition_test;

#[path = "account_macros/amm_pool_state_test.rs"]
pub mod amm_pool_state_test;

#[path = "account_macros/amm_observation_state_test.rs"]
pub mod amm_observation_state_test;

#[path = "account_macros/core_user_record_test.rs"]
pub mod core_user_record_test;

#[path = "account_macros/core_game_session_test.rs"]
pub mod core_game_session_test;

#[path = "account_macros/core_placeholder_record_test.rs"]
pub mod core_placeholder_record_test;
