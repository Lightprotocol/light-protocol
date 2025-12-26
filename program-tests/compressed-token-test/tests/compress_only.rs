// Integration tests for compress_only extension behavior
// Tests for compression and decompression of CToken accounts with Token-2022 extensions.
// These tests verify the compress_only mode behavior for restricted extensions.

#[path = "compress_only/mod.rs"]
mod shared;

// 1. create mint with all restricted extensions
//    - compress and close
//    - decompress
#[path = "compress_only/all.rs"]
mod all;

// 1. create mint with default state set to initialized
//    - compress and close
//    - decompress
// 2. create mint with default state set to frozen
//    - compress and close
//    - decompress
#[path = "compress_only/default_state.rs"]
mod default_state;

// Permanent delegate must be able to decompress
#[path = "compress_only/permanent_delegate.rs"]
mod permanent_delegate;

//
#[path = "compress_only/frozen.rs"]
mod frozen;

// Delegate must be able to decompress
// Delegated value must be the same pre compress and close
#[path = "compress_only/delegated.rs"]
mod delegated;
