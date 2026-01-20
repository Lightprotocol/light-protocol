//! Token 2022 (Token Extensions) program instruction decoder.
//!
//! This module provides a macro-derived decoder for the Token 2022 program,
//! which uses single-byte discriminators based on variant indices.

// Allow the macro-generated code to reference types from this crate
extern crate self as light_instruction_decoder;

use light_instruction_decoder_derive::InstructionDecoder;

/// Token 2022 program instructions.
///
/// The Token 2022 program uses a 1-byte discriminator (variant index).
/// Each variant's discriminator is its position in this enum (0, 1, 2, ...).
///
/// Token 2022 is a superset of SPL Token (indices 0-24 are compatible).
/// Indices 25+ are Token Extensions specific instructions.
///
/// Note: Complex types (Pubkey, COption<Pubkey>, Vec<ExtensionType>) are not
/// fully parsed; only primitive fields are extracted.
#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
    program_name = "Token 2022",
    discriminator_size = 1
)]
pub enum Token2022Instruction {
    // ===== SPL Token compatible instructions (0-24) =====
    /// Initialize a new mint (index 0)
    #[instruction_decoder(account_names = ["mint", "rent"])]
    InitializeMint { decimals: u8 },

    /// Initialize a new token account (index 1)
    #[instruction_decoder(account_names = ["account", "mint", "owner", "rent"])]
    InitializeAccount,

    /// Initialize a multisig account (index 2)
    #[instruction_decoder(account_names = ["multisig", "rent"])]
    InitializeMultisig { m: u8 },

    /// Transfer tokens - DEPRECATED, use TransferChecked (index 3)
    #[instruction_decoder(account_names = ["source", "destination", "authority"])]
    Transfer { amount: u64 },

    /// Approve a delegate (index 4)
    #[instruction_decoder(account_names = ["source", "delegate", "owner"])]
    Approve { amount: u64 },

    /// Revoke delegate authority (index 5)
    #[instruction_decoder(account_names = ["source", "owner"])]
    Revoke,

    /// Set a new authority (index 6)
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
    #[instruction_decoder(account_names = ["account", "mint", "rent"])]
    InitializeAccount2,

    /// Sync native SOL balance (index 17)
    #[instruction_decoder(account_names = ["account"])]
    SyncNative,

    /// Initialize account without rent sysvar (index 18)
    #[instruction_decoder(account_names = ["account", "mint"])]
    InitializeAccount3,

    /// Initialize multisig without rent sysvar (index 19)
    #[instruction_decoder(account_names = ["multisig"])]
    InitializeMultisig2 { m: u8 },

    /// Initialize mint without rent sysvar (index 20)
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
    #[instruction_decoder(account_names = ["mint"])]
    UiAmountToAmount,

    // ===== Token Extensions specific instructions (25+) =====
    /// Initialize mint close authority extension (index 25)
    #[instruction_decoder(account_names = ["mint"])]
    InitializeMintCloseAuthority,

    /// Transfer fee extension instruction prefix (index 26)
    #[instruction_decoder(account_names = ["mint"])]
    TransferFeeExtension,

    /// Confidential transfer extension instruction prefix (index 27)
    #[instruction_decoder(account_names = ["account"])]
    ConfidentialTransferExtension,

    /// Default account state extension instruction prefix (index 28)
    #[instruction_decoder(account_names = ["mint"])]
    DefaultAccountStateExtension,

    /// Reallocate account for extensions (index 29)
    #[instruction_decoder(account_names = ["account", "payer", "system_program"])]
    Reallocate,

    /// Memo transfer extension instruction prefix (index 30)
    #[instruction_decoder(account_names = ["account", "owner"])]
    MemoTransferExtension,

    /// Create the native mint (index 31)
    #[instruction_decoder(account_names = ["mint", "funding_account", "system_program"])]
    CreateNativeMint,

    /// Initialize non-transferable mint extension (index 32)
    #[instruction_decoder(account_names = ["mint"])]
    InitializeNonTransferableMint,

    /// Interest bearing mint extension instruction prefix (index 33)
    #[instruction_decoder(account_names = ["mint"])]
    InterestBearingMintExtension,

    /// CPI guard extension instruction prefix (index 34)
    #[instruction_decoder(account_names = ["account", "owner"])]
    CpiGuardExtension,

    /// Initialize permanent delegate extension (index 35)
    #[instruction_decoder(account_names = ["mint"])]
    InitializePermanentDelegate,

    /// Transfer hook extension instruction prefix (index 36)
    #[instruction_decoder(account_names = ["mint"])]
    TransferHookExtension,

    /// Confidential transfer fee extension instruction prefix (index 37)
    #[instruction_decoder(account_names = ["mint"])]
    ConfidentialTransferFeeExtension,

    /// Withdraw excess lamports from token account (index 38)
    #[instruction_decoder(account_names = ["source", "destination", "authority"])]
    WithdrawExcessLamports,

    /// Metadata pointer extension instruction prefix (index 39)
    #[instruction_decoder(account_names = ["mint"])]
    MetadataPointerExtension,

    /// Group pointer extension instruction prefix (index 40)
    #[instruction_decoder(account_names = ["mint"])]
    GroupPointerExtension,

    /// Group member pointer extension instruction prefix (index 41)
    #[instruction_decoder(account_names = ["mint"])]
    GroupMemberPointerExtension,

    /// Confidential mint/burn extension instruction prefix (index 42)
    #[instruction_decoder(account_names = ["mint"])]
    ConfidentialMintBurnExtension,

    /// Scaled UI amount extension instruction prefix (index 43)
    #[instruction_decoder(account_names = ["mint"])]
    ScaledUiAmountExtension,

    /// Pausable extension instruction prefix (index 44)
    #[instruction_decoder(account_names = ["mint"])]
    PausableExtension,
}
