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

// Per-extension tests (single extension only)
#[path = "compress_only/transfer_fee.rs"]
mod transfer_fee;

#[path = "compress_only/transfer_hook.rs"]
mod transfer_hook;

#[path = "compress_only/pausable.rs"]
mod pausable;

// Failing tests for compression_only requirement
#[path = "compress_only/restricted_required.rs"]
mod restricted_required;

// Failing tests for invalid decompress destination
#[path = "compress_only/invalid_destination.rs"]
mod invalid_destination;

// Failing tests for invalid extension state (non-zero fees, non-nil hook)
#[path = "compress_only/invalid_extension_state.rs"]
mod invalid_extension_state;

// Failing tests:
// 1. cannot decompress to invalid account (try all variants of checked values in validate_decompression_destination)
// 2. cannot compress with restricted extension(s) (try all restricted extensions alone and all combinations)
// 3. extensions in invalid state (transfer hook not nil, transfer fee not zero, etc.)
//
// Functional tests:
// 1. can compress and close -> decompress (all extensions, restricted alone, restricted combinations, no extensions, frozen, delegated)
// 2. randomized (any state (delegated, frozen, token balance 0, token balance > 0), any extension combinations)
