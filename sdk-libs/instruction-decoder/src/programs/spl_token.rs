//! SPL Token program instruction decoder.
//!
//! This module provides a macro-derived decoder for the SPL Token program,
//! which uses single-byte discriminators based on variant indices.

// Allow the macro-generated code to reference types from this crate
extern crate self as light_instruction_decoder;

use light_instruction_decoder_derive::InstructionDecoder;

/// SPL Token program instructions.
///
/// The SPL Token program uses a 1-byte discriminator (variant index).
/// Each variant's discriminator is its position in this enum (0, 1, 2, ...).
///
/// Note: Complex types (Pubkey, COption<Pubkey>) are not fully parsed;
/// only primitive fields are extracted.
#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    program_name = "SPL Token",
    discriminator_size = 1
)]
pub enum SplTokenInstruction {
    /// Initialize a new mint (index 0)
    /// Fields: decimals: u8, mint_authority: Pubkey, freeze_authority: COption<Pubkey>
    #[instruction_decoder(account_names = ["mint", "rent"])]
    InitializeMint { decimals: u8 },

    /// Initialize a new token account (index 1)
    #[instruction_decoder(account_names = ["account", "mint", "owner", "rent"])]
    InitializeAccount,

    /// Initialize a multisig account (index 2)
    #[instruction_decoder(account_names = ["multisig", "rent"])]
    InitializeMultisig { m: u8 },

    /// Transfer tokens (index 3)
    #[instruction_decoder(account_names = ["source", "destination", "authority"])]
    Transfer { amount: u64 },

    /// Approve a delegate (index 4)
    #[instruction_decoder(account_names = ["source", "delegate", "owner"])]
    Approve { amount: u64 },

    /// Revoke delegate authority (index 5)
    #[instruction_decoder(account_names = ["source", "owner"])]
    Revoke,

    /// Set a new authority (index 6)
    /// Fields: authority_type: u8, new_authority: COption<Pubkey>
    #[instruction_decoder(account_names = ["account_or_mint", "current_authority"])]
    SetAuthority { authority_type: u8 },

    /// Mint new tokens (index 7)
    #[instruction_decoder(account_names = ["mint", "destination", "authority"])]
    MintTo { amount: u64 },

    /// Burn tokens (index 8)
    #[instruction_decoder(account_names = ["source", "mint", "authority"])]
    Burn { amount: u64 },

    /// Close a token account (index 9)
    #[instruction_decoder(account_names = ["account", "destination", "authority"])]
    CloseAccount,

    /// Freeze a token account (index 10)
    #[instruction_decoder(account_names = ["account", "mint", "authority"])]
    FreezeAccount,

    /// Thaw a frozen token account (index 11)
    #[instruction_decoder(account_names = ["account", "mint", "authority"])]
    ThawAccount,

    /// Transfer tokens with decimals check (index 12)
    #[instruction_decoder(account_names = ["source", "mint", "destination", "authority"])]
    TransferChecked { amount: u64, decimals: u8 },

    /// Approve delegate with decimals check (index 13)
    #[instruction_decoder(account_names = ["source", "mint", "delegate", "owner"])]
    ApproveChecked { amount: u64, decimals: u8 },

    /// Mint tokens with decimals check (index 14)
    #[instruction_decoder(account_names = ["mint", "destination", "authority"])]
    MintToChecked { amount: u64, decimals: u8 },

    /// Burn tokens with decimals check (index 15)
    #[instruction_decoder(account_names = ["source", "mint", "authority"])]
    BurnChecked { amount: u64, decimals: u8 },

    /// Initialize account with owner in data (index 16)
    /// Fields: owner: Pubkey (32 bytes)
    #[instruction_decoder(account_names = ["account", "mint", "rent"])]
    InitializeAccount2,

    /// Sync native SOL balance (index 17)
    #[instruction_decoder(account_names = ["account"])]
    SyncNative,

    /// Initialize account without rent sysvar (index 18)
    /// Fields: owner: Pubkey (32 bytes)
    #[instruction_decoder(account_names = ["account", "mint"])]
    InitializeAccount3,

    /// Initialize multisig without rent sysvar (index 19)
    #[instruction_decoder(account_names = ["multisig"])]
    InitializeMultisig2 { m: u8 },

    /// Initialize mint without rent sysvar (index 20)
    /// Fields: decimals: u8, mint_authority: Pubkey, freeze_authority: COption<Pubkey>
    #[instruction_decoder(account_names = ["mint"])]
    InitializeMint2 { decimals: u8 },

    /// Get required account size (index 21)
    #[instruction_decoder(account_names = ["mint"])]
    GetAccountDataSize,

    /// Initialize immutable owner extension (index 22)
    #[instruction_decoder(account_names = ["account"])]
    InitializeImmutableOwner,

    /// Convert amount to UI amount string (index 23)
    #[instruction_decoder(account_names = ["mint"])]
    AmountToUiAmount { amount: u64 },

    /// Convert UI amount string to amount (index 24)
    /// Fields: ui_amount: &str (variable length)
    #[instruction_decoder(account_names = ["mint"])]
    UiAmountToAmount,
}
