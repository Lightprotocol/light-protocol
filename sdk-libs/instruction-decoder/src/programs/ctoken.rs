//! Compressed Token (CToken) program instruction decoder.
//!
//! This module provides a macro-derived decoder for the Light Token (CToken) program,
//! which uses non-sequential 1-byte discriminators for Pinocchio instructions.
//!
//! Note: This decoder only handles Pinocchio (1-byte) instructions.
//! Anchor (8-byte) instructions are not decoded by this macro-derived decoder.
//!
//! ## Instruction Data Formats
//!
//! Most CToken instructions have optional max_top_up suffix:
//! - Transfer, MintTo, Burn: 8 bytes (amount) or 10 bytes (amount + max_top_up)
//! - TransferChecked, MintToChecked, BurnChecked: 9 bytes (amount + decimals) or 11 bytes (+ max_top_up)
//! - Approve: 8 bytes (amount) or 10 bytes (amount + max_top_up)
//! - Revoke: 0 bytes or 2 bytes (max_top_up)

// Allow the macro-generated code to reference types from this crate
extern crate self as light_instruction_decoder;

use light_instruction_decoder_derive::InstructionDecoder;
use light_token_interface::instructions::{
    mint_action::MintActionCompressedInstructionData,
    transfer2::CompressedTokenInstructionDataTransfer2,
};
use solana_instruction::AccountMeta;

/// Standard token accounts (before packed_accounts).
/// Transfer2 has 10 fixed accounts at indices 0-9.
const PACKED_ACCOUNTS_START: usize = 10;

/// Format Transfer2 instruction data with resolved pubkeys.
///
/// This formatter provides a human-readable view of the transfer instruction,
/// resolving account indices to actual pubkeys from the instruction accounts.
///
/// Mode detection:
/// - CPI context mode (cpi_context is Some): Packed accounts are passed via CPI context account,
///   not in the instruction's accounts array. Shows raw indices only.
/// - Direct mode (cpi_context is None): Packed accounts are in the accounts array at
///   PACKED_ACCOUNTS_START offset. Resolves indices to actual pubkeys.
///
/// Index resolution:
/// - In CPI context mode: all indices shown as packed[N] (stored in CPI context account)
/// - In direct mode: all indices (owner, mint, delegate, merkle_tree, queue) are resolved
///   using PACKED_ACCOUNTS_START offset. Note: this assumes a specific account layout and
///   may show OUT_OF_BOUNDS if the actual layout differs.
#[cfg(not(target_os = "solana"))]
pub fn format_transfer2(
    data: &CompressedTokenInstructionDataTransfer2,
    accounts: &[AccountMeta],
) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    // Determine if packed accounts are in CPI context (not directly in accounts array)
    // When cpi_context is Some, packed accounts are stored in/read from a CPI context account
    let uses_cpi_context = data.cpi_context.is_some();

    // Helper to resolve account index
    // In CPI context mode: all indices are packed indices stored in CPI context
    // In direct mode: packed indices are offset by PACKED_ACCOUNTS_START
    let resolve = |index: u8| -> String {
        if uses_cpi_context {
            // All accounts (including trees/queues) are in CPI context
            format!("packed[{}]", index)
        } else {
            accounts
                .get(PACKED_ACCOUNTS_START + index as usize)
                .map(|a| a.pubkey.to_string())
                .unwrap_or_else(|| {
                    format!("OUT_OF_BOUNDS({})", PACKED_ACCOUNTS_START + index as usize)
                })
        }
    };

    // Header with mode indicator
    if uses_cpi_context {
        let _ = writeln!(
            output,
            "[CPI Context Mode - packed accounts in CPI context]"
        );
    }

    // Top-level fields
    let _ = writeln!(output, "output_queue: {}", resolve(data.output_queue));
    if data.max_top_up > 0 {
        let _ = writeln!(output, "max_top_up: {}", data.max_top_up);
    }
    if data.with_transaction_hash {
        let _ = writeln!(output, "with_transaction_hash: true");
    }

    // Input tokens
    let _ = writeln!(output, "Input Tokens ({}):", data.in_token_data.len());
    for (i, token) in data.in_token_data.iter().enumerate() {
        let _ = writeln!(output, "  [{}]", i);
        let _ = writeln!(output, "    owner: {}", resolve(token.owner));
        let _ = writeln!(output, "    mint: {}", resolve(token.mint));
        let _ = writeln!(output, "    amount: {}", token.amount);
        if token.has_delegate {
            let _ = writeln!(output, "    delegate: {}", resolve(token.delegate));
        }
        let _ = writeln!(output, "    version: {}", token.version);
        // Merkle context
        let _ = writeln!(
            output,
            "    merkle_tree: {}",
            resolve(token.merkle_context.merkle_tree_pubkey_index)
        );
        let _ = writeln!(
            output,
            "    queue: {}",
            resolve(token.merkle_context.queue_pubkey_index)
        );
        let _ = writeln!(
            output,
            "    leaf_index: {}",
            token.merkle_context.leaf_index
        );
        let _ = writeln!(output, "    root_index: {}", token.root_index);
    }

    // Output tokens
    let _ = writeln!(output, "Output Tokens ({}):", data.out_token_data.len());
    for (i, token) in data.out_token_data.iter().enumerate() {
        let _ = writeln!(output, "  [{}]", i);
        let _ = writeln!(output, "    owner: {}", resolve(token.owner));
        let _ = writeln!(output, "    mint: {}", resolve(token.mint));
        let _ = writeln!(output, "    amount: {}", token.amount);
        if token.has_delegate {
            let _ = writeln!(output, "    delegate: {}", resolve(token.delegate));
        }
        let _ = writeln!(output, "    version: {}", token.version);
    }

    // Compressions if present
    if let Some(compressions) = &data.compressions {
        let _ = writeln!(output, "Compressions ({}):", compressions.len());
        for (i, comp) in compressions.iter().enumerate() {
            let _ = writeln!(output, "  [{}]", i);
            let _ = writeln!(output, "    mode: {:?}", comp.mode);
            let _ = writeln!(output, "    amount: {}", comp.amount);
            let _ = writeln!(output, "    mint: {}", resolve(comp.mint));
            let _ = writeln!(
                output,
                "    source_or_recipient: {}",
                resolve(comp.source_or_recipient)
            );
            let _ = writeln!(output, "    authority: {}", resolve(comp.authority));
        }
    }

    output
}

/// Format MintAction instruction data with resolved pubkeys.
///
/// This formatter provides a human-readable view of the mint action instruction,
/// resolving account indices to actual pubkeys from the instruction accounts.
///
/// Mode detection:
/// - CPI context mode (cpi_context.set_context || first_set_context): Packed accounts are passed
///   via CPI context account, not in the instruction's accounts array. Shows raw indices only.
/// - Direct mode: Packed accounts are in the accounts array at PACKED_ACCOUNTS_START offset.
///   Resolves indices to actual pubkeys.
#[cfg(not(target_os = "solana"))]
pub fn format_mint_action(
    data: &MintActionCompressedInstructionData,
    accounts: &[AccountMeta],
) -> String {
    use std::fmt::Write;

    use light_token_interface::instructions::mint_action::Action;
    let mut output = String::new();

    // CPI context mode: set_context OR first_set_context means packed accounts in CPI context
    let uses_cpi_context = data
        .cpi_context
        .as_ref()
        .map(|ctx| ctx.set_context || ctx.first_set_context)
        .unwrap_or(false);

    // Helper to resolve account index
    let resolve = |index: u8| -> String {
        if uses_cpi_context {
            format!("packed[{}]", index)
        } else {
            accounts
                .get(PACKED_ACCOUNTS_START + index as usize)
                .map(|a| a.pubkey.to_string())
                .unwrap_or_else(|| {
                    format!("OUT_OF_BOUNDS({})", PACKED_ACCOUNTS_START + index as usize)
                })
        }
    };

    // Header with mode indicator
    if uses_cpi_context {
        let _ = writeln!(
            output,
            "[CPI Context Mode - packed accounts in CPI context]"
        );
    }

    // Top-level fields
    if data.create_mint.is_some() {
        let _ = writeln!(output, "create_mint: true");
    } else {
        let _ = writeln!(output, "leaf_index: {}", data.leaf_index);
        if data.prove_by_index {
            let _ = writeln!(output, "prove_by_index: true");
        }
    }
    let _ = writeln!(output, "root_index: {}", data.root_index);
    if data.max_top_up > 0 {
        let _ = writeln!(output, "max_top_up: {}", data.max_top_up);
    }

    // Mint data summary (if present)
    if let Some(mint) = &data.mint {
        let _ = writeln!(output, "Mint:");
        let _ = writeln!(output, "  supply: {}", mint.supply);
        let _ = writeln!(output, "  decimals: {}", mint.decimals);
        if let Some(auth) = &mint.mint_authority {
            let _ = writeln!(
                output,
                "  mint_authority: {}",
                bs58::encode(auth).into_string()
            );
        }
        if let Some(auth) = &mint.freeze_authority {
            let _ = writeln!(
                output,
                "  freeze_authority: {}",
                bs58::encode(auth).into_string()
            );
        }
        if let Some(exts) = &mint.extensions {
            let _ = writeln!(output, "  extensions: {}", exts.len());
        }
    }

    // Actions
    let _ = writeln!(output, "Actions ({}):", data.actions.len());
    for (i, action) in data.actions.iter().enumerate() {
        match action {
            Action::MintToCompressed(a) => {
                let _ = writeln!(output, "  [{}] MintToCompressed:", i);
                let _ = writeln!(output, "    version: {}", a.token_account_version);
                for (j, r) in a.recipients.iter().enumerate() {
                    let _ = writeln!(
                        output,
                        "    recipient[{}]: {} amount: {}",
                        j,
                        bs58::encode(&r.recipient).into_string(),
                        r.amount
                    );
                }
            }
            Action::UpdateMintAuthority(a) => {
                let authority_str = a
                    .new_authority
                    .as_ref()
                    .map(|p| bs58::encode(p).into_string())
                    .unwrap_or_else(|| "None".to_string());
                let _ = writeln!(output, "  [{}] UpdateMintAuthority: {}", i, authority_str);
            }
            Action::UpdateFreezeAuthority(a) => {
                let authority_str = a
                    .new_authority
                    .as_ref()
                    .map(|p| bs58::encode(p).into_string())
                    .unwrap_or_else(|| "None".to_string());
                let _ = writeln!(output, "  [{}] UpdateFreezeAuthority: {}", i, authority_str);
            }
            Action::MintTo(a) => {
                let _ = writeln!(
                    output,
                    "  [{}] MintTo: account: {}, amount: {}",
                    i,
                    resolve(a.account_index),
                    a.amount
                );
            }
            Action::UpdateMetadataField(a) => {
                let field_name = match a.field_type {
                    0 => "Name",
                    1 => "Symbol",
                    2 => "Uri",
                    _ => "Custom",
                };
                let _ = writeln!(
                    output,
                    "  [{}] UpdateMetadataField: ext[{}] {} = {:?}",
                    i,
                    a.extension_index,
                    field_name,
                    String::from_utf8_lossy(&a.value)
                );
            }
            Action::UpdateMetadataAuthority(a) => {
                let _ = writeln!(
                    output,
                    "  [{}] UpdateMetadataAuthority: ext[{}] = {}",
                    i,
                    a.extension_index,
                    bs58::encode(&a.new_authority).into_string()
                );
            }
            Action::RemoveMetadataKey(a) => {
                let _ = writeln!(
                    output,
                    "  [{}] RemoveMetadataKey: ext[{}] key={:?} idempotent={}",
                    i,
                    a.extension_index,
                    String::from_utf8_lossy(&a.key),
                    a.idempotent != 0
                );
            }
            Action::DecompressMint(a) => {
                let _ = writeln!(
                    output,
                    "  [{}] DecompressMint: rent_payment={} write_top_up={}",
                    i, a.rent_payment, a.write_top_up
                );
            }
            Action::CompressAndCloseMint(a) => {
                let _ = writeln!(
                    output,
                    "  [{}] CompressAndCloseMint: idempotent={}",
                    i,
                    a.idempotent != 0
                );
            }
        }
    }

    // CPI context details (if present)
    if let Some(ctx) = &data.cpi_context {
        let _ = writeln!(output, "CPI Context:");
        let _ = writeln!(
            output,
            "  mode: {}",
            if ctx.first_set_context {
                "first_set_context"
            } else if ctx.set_context {
                "set_context"
            } else {
                "read"
            }
        );
        let _ = writeln!(output, "  in_tree: packed[{}]", ctx.in_tree_index);
        let _ = writeln!(output, "  in_queue: packed[{}]", ctx.in_queue_index);
        let _ = writeln!(output, "  out_queue: packed[{}]", ctx.out_queue_index);
        if ctx.token_out_queue_index > 0 {
            let _ = writeln!(
                output,
                "  token_out_queue: packed[{}]",
                ctx.token_out_queue_index
            );
        }
        let _ = writeln!(
            output,
            "  address_tree: {}",
            bs58::encode(&ctx.address_tree_pubkey).into_string()
        );
    }

    output
}

/// Compressed Token (CToken) program instructions.
///
/// The CToken program uses non-sequential 1-byte discriminators.
/// Each variant has an explicit #[discriminator = N] attribute.
///
/// Field definitions show the base required fields; max_top_up is optional.
#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m",
    program_name = "Light Token",
    discriminator_size = 1
)]
pub enum CTokenInstruction {
    /// Transfer compressed tokens (discriminator 3)
    /// Data: amount (u64) [+ max_top_up (u16)]
    #[discriminator = 3]
    #[instruction_decoder(account_names = ["source", "destination", "authority"])]
    Transfer { amount: u64 },

    /// Approve delegate for compressed tokens (discriminator 4)
    /// Data: amount (u64) [+ max_top_up (u16)]
    #[discriminator = 4]
    #[instruction_decoder(account_names = ["source", "delegate", "owner"])]
    Approve { amount: u64 },

    /// Revoke delegate authority (discriminator 5)
    /// Data: [max_top_up (u16)]
    #[discriminator = 5]
    #[instruction_decoder(account_names = ["source", "owner"])]
    Revoke,

    /// Mint compressed tokens to an account (discriminator 7)
    /// Data: amount (u64) [+ max_top_up (u16)]
    #[discriminator = 7]
    #[instruction_decoder(account_names = ["cmint", "destination", "authority"])]
    MintTo { amount: u64 },

    /// Burn compressed tokens (discriminator 8)
    /// Data: amount (u64) [+ max_top_up (u16)]
    #[discriminator = 8]
    #[instruction_decoder(account_names = ["source", "cmint", "authority"])]
    Burn { amount: u64 },

    /// Close a compressed token account (discriminator 9)
    #[discriminator = 9]
    #[instruction_decoder(account_names = ["account", "destination", "authority"])]
    CloseTokenAccount,

    /// Freeze a compressed token account (discriminator 10)
    #[discriminator = 10]
    #[instruction_decoder(account_names = ["account", "mint", "authority"])]
    FreezeAccount,

    /// Thaw a frozen compressed token account (discriminator 11)
    #[discriminator = 11]
    #[instruction_decoder(account_names = ["account", "mint", "authority"])]
    ThawAccount,

    /// Transfer compressed tokens with decimals check (discriminator 12)
    /// Data: amount (u64) + decimals (u8) [+ max_top_up (u16)]
    #[discriminator = 12]
    #[instruction_decoder(account_names = ["source", "mint", "destination", "authority"])]
    TransferChecked { amount: u64, decimals: u8 },

    /// Mint compressed tokens with decimals check (discriminator 14)
    /// Data: amount (u64) + decimals (u8) [+ max_top_up (u16)]
    #[discriminator = 14]
    #[instruction_decoder(account_names = ["cmint", "destination", "authority"])]
    MintToChecked { amount: u64, decimals: u8 },

    /// Burn compressed tokens with decimals check (discriminator 15)
    /// Data: amount (u64) + decimals (u8) [+ max_top_up (u16)]
    #[discriminator = 15]
    #[instruction_decoder(account_names = ["source", "cmint", "authority"])]
    BurnChecked { amount: u64, decimals: u8 },

    /// Create a new compressed token account (discriminator 18)
    #[discriminator = 18]
    #[instruction_decoder(account_names = ["token_account", "mint", "payer", "config", "system_program", "rent_payer"])]
    CreateTokenAccount,

    /// Create an associated compressed token account (discriminator 100)
    #[discriminator = 100]
    #[instruction_decoder(account_names = ["owner", "mint", "fee_payer", "ata", "system_program", "config", "rent_payer"])]
    CreateAssociatedTokenAccount,

    /// Transfer v2 with additional options (discriminator 101)
    #[discriminator = 101]
    #[instruction_decoder(
        account_names = ["fee_payer", "authority", "registered_program_pda", "noop_program", "account_compression_authority", "account_compression_program", "self_program", "cpi_signer", "light_system_program", "system_program"],
        params = CompressedTokenInstructionDataTransfer2,
        pretty_formatter = crate::programs::ctoken::format_transfer2
    )]
    Transfer2,

    /// Create associated token account idempotently (discriminator 102)
    #[discriminator = 102]
    #[instruction_decoder(account_names = ["owner", "mint", "fee_payer", "ata", "system_program", "config", "rent_payer"])]
    CreateAssociatedTokenAccountIdempotent,

    /// Mint action for compressed tokens (discriminator 103)
    #[discriminator = 103]
    #[instruction_decoder(
        account_names = ["fee_payer", "authority", "registered_program_pda", "noop_program", "account_compression_authority", "account_compression_program", "self_program", "cpi_signer", "light_system_program", "system_program"],
        params = MintActionCompressedInstructionData,
        pretty_formatter = crate::programs::ctoken::format_mint_action
    )]
    MintAction,

    /// Claim compressed tokens (discriminator 104)
    #[discriminator = 104]
    #[instruction_decoder(account_names = ["forester", "ctoken_account", "rent_recipient", "config"])]
    Claim,

    /// Withdraw from funding pool (discriminator 105)
    #[discriminator = 105]
    #[instruction_decoder(account_names = ["authority", "rent_recipient", "config", "destination"])]
    WithdrawFundingPool,
}
