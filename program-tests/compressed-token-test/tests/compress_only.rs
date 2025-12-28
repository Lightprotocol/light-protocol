//! Integration tests for compress_only extension behavior.
//!
//! Tests for compression and decompression of CToken accounts with Token-2022 extensions.
//! These tests verify the compress_only mode behavior for restricted extensions.
//!
//! ## Test Coverage (see .claude/test-coverage/transfer2-compress-and-close-tests.md)
//!
//! ### Compress restricted mints
//! - #4, #5: Cannot compress to compressed token account (covered in ctoken/extensions.rs)
//!
//! ### CompressAndClose
//! - #6: Frozen account can be compressed and closed (frozen.rs)
//! - #7: Delegated account can be compressed and closed (delegated.rs)
//! - #8: Paused mint can be compressed and closed (pausable.rs)
//! - #9: Non-zero transfer fee mint can be compressed and closed (transfer_fee.rs)
//! - #10: Non-nil transfer hook mint can be compressed and closed (transfer_hook.rs)
//! - #11: CompressedOnly extension required for restricted mints (restricted_required.rs)
//! - #12: Orphan delegate preserved (orphan_delegate.rs)
//!
//! ### Decompress
//! - #13: Can only decompress to ctoken (decompress_restrictions.rs)
//! - #14: Must decompress complete account (decompress_restrictions.rs)
//! - #15: Restores frozen state (frozen.rs)
//! - #16: Restores delegate and delegated_amount (delegated.rs)
//! - #17: Restores orphan delegate (orphan_delegate.rs)
//! - #18: Owner can decompress (all.rs)
//! - #19: Delegate can decompress (delegated.rs)
//! - #20: Permanent delegate can decompress (permanent_delegate.rs)
//! - #21-23: Decompress succeeds with paused/fee/hook extensions (pausable.rs, transfer_fee.rs, transfer_hook.rs)
//!
//! ### Round-trip
//! - #24: Full round-trip frozen (frozen.rs)
//! - #25: Full round-trip delegated (delegated.rs)
//! - #26: Full round-trip orphan delegate (orphan_delegate.rs)
//! - #27: Full round-trip withheld_transfer_fee (withheld_fee.rs)
//! - #28: close_authority - NOT SUPPORTED (not in CompressedOnlyExtensionInstructionData)
//!
//! ### Negative tests
//! - #29-32: Mismatch validation - NOT TESTABLE (registry always builds correct out_tlv)
//! - #33: Decompress fails to non-fresh destination (invalid_destination.rs)

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

// Withheld transfer fee preservation through compress/decompress
#[path = "compress_only/withheld_fee.rs"]
mod withheld_fee;

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

// Failing tests for CompressedOnly decompress restrictions
// - Cannot decompress to SPL Token-2022 account (must use CToken)
// - Cannot do partial decompress (would create change output)
#[path = "compress_only/decompress_restrictions.rs"]
mod decompress_restrictions;

// Failing tests:
// 1. cannot decompress to invalid account (try all variants of checked values in validate_decompression_destination)
// 2. cannot compress with restricted extension(s) (try all restricted extensions alone and all combinations)
// 3. extensions in invalid state (transfer hook not nil, transfer fee not zero, etc.)
//
// Functional tests:
// 1. can compress and close -> decompress (all extensions, restricted alone, restricted combinations, no extensions, frozen, delegated)
// 2. randomized (any state (delegated, frozen, token balance 0, token balance > 0), any extension combinations)
