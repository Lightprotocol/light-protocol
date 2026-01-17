//! D1: Field type variations
//!
//! Tests `is_pubkey_type()`, `is_copy_type()`, and Pack generation code paths.

pub mod no_pubkey;
pub mod single_pubkey;
pub mod multi_pubkey;
pub mod non_copy;
pub mod option_pubkey;
pub mod option_primitive;
pub mod arrays;
pub mod all;
