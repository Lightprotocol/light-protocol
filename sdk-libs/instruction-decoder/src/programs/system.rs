//! Solana System Program instruction decoder.
//!
//! This module provides a macro-derived decoder for the Solana System Program,
//! which uses 4-byte (u32) discriminators for instruction types.

// Allow the macro-generated code to reference types from this crate
extern crate self as light_instruction_decoder;

use light_instruction_decoder_derive::InstructionDecoder;

/// Solana System Program instructions.
///
/// The System Program uses a 4-byte discriminator (u32 little-endian).
/// Each variant's discriminator is its position in this enum (0, 1, 2, ...).
#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "11111111111111111111111111111111",
    program_name = "System Program",
    discriminator_size = 4
)]
pub enum SystemInstruction {
    /// Create a new account (index 0)
    /// Data: lamports (u64) + space (u64) + owner (Pubkey)
    #[instruction_decoder(account_names = ["funding_account", "new_account"])]
    CreateAccount { lamports: u64, space: u64 },

    /// Assign account to a program (index 1)
    /// Data: owner (Pubkey)
    #[instruction_decoder(account_names = ["account"])]
    Assign,

    /// Transfer lamports (index 2)
    /// Data: lamports (u64)
    #[instruction_decoder(account_names = ["from", "to"])]
    Transfer { lamports: u64 },

    /// Create account with seed (index 3)
    /// Data: base (Pubkey) + seed (String) + lamports (u64) + space (u64) + owner (Pubkey)
    #[instruction_decoder(account_names = ["funding_account", "created_account", "base_account"])]
    CreateAccountWithSeed { lamports: u64, space: u64 },

    /// Advance nonce account (index 4)
    #[instruction_decoder(account_names = ["nonce_account", "recent_blockhashes_sysvar", "nonce_authority"])]
    AdvanceNonceAccount,

    /// Withdraw from nonce account (index 5)
    /// Data: lamports (u64)
    #[instruction_decoder(account_names = ["nonce_account", "recipient", "recent_blockhashes_sysvar", "rent_sysvar", "nonce_authority"])]
    WithdrawNonceAccount { lamports: u64 },

    /// Initialize nonce account (index 6)
    /// Data: authority (Pubkey)
    #[instruction_decoder(account_names = ["nonce_account", "recent_blockhashes_sysvar", "rent_sysvar"])]
    InitializeNonceAccount,

    /// Authorize nonce account (index 7)
    /// Data: new_authority (Pubkey)
    #[instruction_decoder(account_names = ["nonce_account", "nonce_authority"])]
    AuthorizeNonceAccount,

    /// Allocate space for account (index 8)
    /// Data: space (u64)
    #[instruction_decoder(account_names = ["account"])]
    Allocate { space: u64 },

    /// Allocate space with seed (index 9)
    /// Data: base (Pubkey) + seed (String) + space (u64) + owner (Pubkey)
    #[instruction_decoder(account_names = ["account", "base_account"])]
    AllocateWithSeed { space: u64 },

    /// Assign account with seed (index 10)
    /// Data: base (Pubkey) + seed (String) + owner (Pubkey)
    #[instruction_decoder(account_names = ["account", "base_account"])]
    AssignWithSeed,

    /// Transfer with seed (index 11)
    /// Data: lamports (u64) + from_seed (String) + from_owner (Pubkey)
    #[instruction_decoder(account_names = ["funding_account", "base_account", "recipient"])]
    TransferWithSeed { lamports: u64 },

    /// Upgrade nonce account (index 12)
    #[instruction_decoder(account_names = ["nonce_account"])]
    UpgradeNonceAccount,
}
