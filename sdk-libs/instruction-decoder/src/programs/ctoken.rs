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

/// Calculate the packed accounts start position for Transfer2.
///
/// The start position depends on the instruction path and optional accounts:
/// - Path A (compressions-only): start = 2 (cpi_authority_pda, fee_payer)
/// - Path B (CPI context write): start = 4 (light_system, fee_payer, cpi_authority, cpi_context)
/// - Path C (full transfer): start = 7 + optional accounts
///   - +1 for sol_pool_pda (when lamports imbalance)
///   - +1 for sol_decompression_recipient (when decompressing SOL)
///   - +1 for cpi_context_account (when cpi_context present but not writing)
#[cfg(not(target_os = "solana"))]
fn calculate_packed_accounts_start(data: &CompressedTokenInstructionDataTransfer2) -> usize {
    let no_compressed_accounts = data.in_token_data.is_empty() && data.out_token_data.is_empty();
    let cpi_context_write_required = data
        .cpi_context
        .as_ref()
        .map(|ctx| ctx.set_context || ctx.first_set_context)
        .unwrap_or(false);

    if no_compressed_accounts {
        // Path A: compressions-only
        // [cpi_authority_pda, fee_payer, ...packed_accounts]
        2
    } else if cpi_context_write_required {
        // Path B: CPI context write
        // [light_system_program, fee_payer, cpi_authority_pda, cpi_context]
        // No packed accounts in this path (return 4 to indicate end of accounts)
        4
    } else {
        // Path C: Full transfer
        // Base: [light_system_program, fee_payer, cpi_authority_pda, registered_program_pda,
        //        account_compression_authority, account_compression_program, system_program]
        let mut start = 7;

        // Optional: sol_pool_pda (when lamports imbalance exists)
        let in_lamports: u64 = data
            .in_lamports
            .as_ref()
            .map(|v| v.iter().sum())
            .unwrap_or(0);
        let out_lamports: u64 = data
            .out_lamports
            .as_ref()
            .map(|v| v.iter().sum())
            .unwrap_or(0);
        if in_lamports != out_lamports {
            start += 1; // sol_pool_pda
        }

        // Optional: sol_decompression_recipient (when decompressing SOL)
        if out_lamports > in_lamports {
            start += 1; // sol_decompression_recipient
        }

        // Optional: cpi_context_account (when cpi_context present but not writing)
        if data.cpi_context.is_some() {
            start += 1; // cpi_context_account
        }

        start
    }
}

/// Format Transfer2 instruction data with resolved pubkeys.
///
/// This formatter provides a human-readable view of the transfer instruction,
/// resolving account indices to actual pubkeys from the instruction accounts.
///
/// Mode detection:
/// - CPI context mode (cpi_context.set_context || first_set_context): Shows raw indices
/// - Direct mode: Resolves packed account indices using dynamically calculated start position
#[cfg(not(target_os = "solana"))]
pub fn format_transfer2(
    data: &CompressedTokenInstructionDataTransfer2,
    accounts: &[AccountMeta],
) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    // Determine if packed accounts are in CPI context write mode
    let cpi_context_write_mode = data
        .cpi_context
        .as_ref()
        .map(|ctx| ctx.set_context || ctx.first_set_context)
        .unwrap_or(false);

    // Calculate where packed accounts start based on instruction path
    let packed_accounts_start = calculate_packed_accounts_start(data);

    // Helper to resolve account index
    let resolve = |index: u8| -> String {
        if cpi_context_write_mode {
            // All accounts are in CPI context
            format!("packed[{}]", index)
        } else {
            accounts
                .get(packed_accounts_start + index as usize)
                .map(|a| a.pubkey.to_string())
                .unwrap_or_else(|| format!("OUT_OF_BOUNDS({})", index))
        }
    };

    // Header with mode indicator
    if cpi_context_write_mode {
        let _ = writeln!(
            output,
            "[CPI Context Write Mode - packed accounts in CPI context]"
        );
    }

    // Top-level fields
    let _ = writeln!(output, "output_queue: {}", resolve(data.output_queue));
    if data.max_top_up != u16::MAX {
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

/// Resolve Transfer2 account names dynamically based on instruction data.
///
/// Transfer2 has a dynamic account layout with three mutually exclusive paths:
///
/// **Path A: Compressions-only** (`in_token_data.is_empty() && out_token_data.is_empty()`)
/// - Account 0: `compressions_only_cpi_authority_pda`
/// - Account 1: `compressions_only_fee_payer`
/// - Remaining: packed_accounts
///
/// **Path B: CPI Context Write** (`cpi_context.set_context || cpi_context.first_set_context`)
/// - Account 0: `light_system_program`
/// - Account 1: `fee_payer`
/// - Account 2: `cpi_authority_pda`
/// - Account 3: `cpi_context`
/// - No packed accounts
///
/// **Path C: Full Transfer** (default)
/// - 7 fixed accounts: light_system_program, fee_payer, cpi_authority_pda, registered_program_pda,
///   account_compression_authority, account_compression_program, system_program
/// - Optional: sol_pool_pda (when lamports imbalance exists)
/// - Optional: sol_decompression_recipient (when decompressing SOL)
/// - Optional: cpi_context_account (when cpi_context is present but not writing)
/// - Remaining: packed_accounts
#[cfg(not(target_os = "solana"))]
pub fn resolve_transfer2_account_names(
    data: &CompressedTokenInstructionDataTransfer2,
    accounts: &[AccountMeta],
) -> Vec<String> {
    use std::collections::HashMap;

    let mut names = Vec::with_capacity(accounts.len());
    let mut idx = 0;
    let mut known_pubkeys: HashMap<[u8; 32], String> = HashMap::new();

    let mut add_name = |name: &str,
                        accounts: &[AccountMeta],
                        idx: &mut usize,
                        known: &mut HashMap<[u8; 32], String>| {
        if *idx < accounts.len() {
            names.push(name.to_string());
            known.insert(accounts[*idx].pubkey.to_bytes(), name.to_string());
            *idx += 1;
            true
        } else {
            false
        }
    };

    // Determine path from instruction data
    let no_compressed_accounts = data.in_token_data.is_empty() && data.out_token_data.is_empty();
    let cpi_context_write_required = data
        .cpi_context
        .as_ref()
        .map(|ctx| ctx.set_context || ctx.first_set_context)
        .unwrap_or(false);

    if no_compressed_accounts {
        // Path A: Compressions-only
        add_name(
            "compressions_only_cpi_authority_pda",
            accounts,
            &mut idx,
            &mut known_pubkeys,
        );
        add_name(
            "compressions_only_fee_payer",
            accounts,
            &mut idx,
            &mut known_pubkeys,
        );
    } else if cpi_context_write_required {
        // Path B: CPI Context Write
        add_name(
            "light_system_program",
            accounts,
            &mut idx,
            &mut known_pubkeys,
        );
        add_name("fee_payer", accounts, &mut idx, &mut known_pubkeys);
        add_name("cpi_authority_pda", accounts, &mut idx, &mut known_pubkeys);
        add_name("cpi_context", accounts, &mut idx, &mut known_pubkeys);
        // No packed accounts in this path
        return names;
    } else {
        // Path C: Full Transfer
        add_name(
            "light_system_program",
            accounts,
            &mut idx,
            &mut known_pubkeys,
        );
        add_name("fee_payer", accounts, &mut idx, &mut known_pubkeys);
        add_name("cpi_authority_pda", accounts, &mut idx, &mut known_pubkeys);
        add_name(
            "registered_program_pda",
            accounts,
            &mut idx,
            &mut known_pubkeys,
        );
        add_name(
            "account_compression_authority",
            accounts,
            &mut idx,
            &mut known_pubkeys,
        );
        add_name(
            "account_compression_program",
            accounts,
            &mut idx,
            &mut known_pubkeys,
        );
        add_name("system_program", accounts, &mut idx, &mut known_pubkeys);

        // Optional accounts - determine from instruction data
        // sol_pool_pda: when lamports imbalance exists
        let in_lamports: u64 = data
            .in_lamports
            .as_ref()
            .map(|v| v.iter().sum())
            .unwrap_or(0);
        let out_lamports: u64 = data
            .out_lamports
            .as_ref()
            .map(|v| v.iter().sum())
            .unwrap_or(0);
        let with_sol_pool = in_lamports != out_lamports;
        if with_sol_pool {
            add_name("sol_pool_pda", accounts, &mut idx, &mut known_pubkeys);
        }

        // sol_decompression_recipient: when decompressing SOL (out > in)
        let with_sol_decompression = out_lamports > in_lamports;
        if with_sol_decompression {
            add_name(
                "sol_decompression_recipient",
                accounts,
                &mut idx,
                &mut known_pubkeys,
            );
        }

        // cpi_context_account: add placeholder - formatter will use transaction-level name
        if data.cpi_context.is_some() {
            names.push(String::new()); // Empty = use formatter's KNOWN_ACCOUNTS lookup
            idx += 1;
        }
    }

    // Build a map of packed account index -> role name from instruction data
    let mut packed_roles: HashMap<u8, String> = HashMap::new();
    let mut owner_count = 0u8;
    let mut mint_count = 0u8;
    let mut delegate_count = 0u8;
    let mut in_merkle_count = 0u8;
    let mut in_queue_count = 0u8;
    let mut compress_mint_count = 0u8;
    let mut compress_source_count = 0u8;
    let mut compress_auth_count = 0u8;

    // output_queue
    packed_roles
        .entry(data.output_queue)
        .or_insert_with(|| "output_queue".to_string());

    // Input token data
    for token in data.in_token_data.iter() {
        packed_roles.entry(token.owner).or_insert_with(|| {
            let name = if owner_count == 0 {
                "owner".to_string()
            } else {
                format!("owner_{}", owner_count)
            };
            owner_count = owner_count.saturating_add(1);
            name
        });
        packed_roles.entry(token.mint).or_insert_with(|| {
            let name = if mint_count == 0 {
                "mint".to_string()
            } else {
                format!("mint_{}", mint_count)
            };
            mint_count = mint_count.saturating_add(1);
            name
        });
        if token.has_delegate {
            packed_roles.entry(token.delegate).or_insert_with(|| {
                let name = if delegate_count == 0 {
                    "delegate".to_string()
                } else {
                    format!("delegate_{}", delegate_count)
                };
                delegate_count = delegate_count.saturating_add(1);
                name
            });
        }
        packed_roles
            .entry(token.merkle_context.merkle_tree_pubkey_index)
            .or_insert_with(|| {
                let name = if in_merkle_count == 0 {
                    "in_merkle_tree".to_string()
                } else {
                    format!("in_merkle_tree_{}", in_merkle_count)
                };
                in_merkle_count = in_merkle_count.saturating_add(1);
                name
            });
        packed_roles
            .entry(token.merkle_context.queue_pubkey_index)
            .or_insert_with(|| {
                let name = if in_queue_count == 0 {
                    "in_nullifier_queue".to_string()
                } else {
                    format!("in_nullifier_queue_{}", in_queue_count)
                };
                in_queue_count = in_queue_count.saturating_add(1);
                name
            });
    }

    // Output token data
    for token in data.out_token_data.iter() {
        packed_roles.entry(token.owner).or_insert_with(|| {
            let name = if owner_count == 0 {
                "owner".to_string()
            } else {
                format!("owner_{}", owner_count)
            };
            owner_count = owner_count.saturating_add(1);
            name
        });
        packed_roles.entry(token.mint).or_insert_with(|| {
            let name = if mint_count == 0 {
                "mint".to_string()
            } else {
                format!("mint_{}", mint_count)
            };
            mint_count = mint_count.saturating_add(1);
            name
        });
        if token.has_delegate {
            packed_roles.entry(token.delegate).or_insert_with(|| {
                let name = if delegate_count == 0 {
                    "delegate".to_string()
                } else {
                    format!("delegate_{}", delegate_count)
                };
                delegate_count = delegate_count.saturating_add(1);
                name
            });
        }
    }

    // Compressions
    if let Some(compressions) = &data.compressions {
        for comp in compressions.iter() {
            packed_roles.entry(comp.mint).or_insert_with(|| {
                let name = if compress_mint_count == 0 {
                    "compress_mint".to_string()
                } else {
                    format!("compress_mint_{}", compress_mint_count)
                };
                compress_mint_count = compress_mint_count.saturating_add(1);
                name
            });
            packed_roles
                .entry(comp.source_or_recipient)
                .or_insert_with(|| {
                    let name = if compress_source_count == 0 {
                        "compress_source".to_string()
                    } else {
                        format!("compress_source_{}", compress_source_count)
                    };
                    compress_source_count = compress_source_count.saturating_add(1);
                    name
                });
            packed_roles.entry(comp.authority).or_insert_with(|| {
                let name = if compress_auth_count == 0 {
                    "compress_authority".to_string()
                } else {
                    format!("compress_authority_{}", compress_auth_count)
                };
                compress_auth_count = compress_auth_count.saturating_add(1);
                name
            });
        }
    }

    // Remaining accounts are packed - prioritize role names from instruction data
    let mut packed_idx: u8 = 0;
    while idx < accounts.len() {
        let pubkey_bytes = accounts[idx].pubkey.to_bytes();

        // First check if we have a semantic role from instruction data
        if let Some(role) = packed_roles.get(&packed_idx) {
            // Use the role name, and note if it matches a known account
            if let Some(known_name) = known_pubkeys.get(&pubkey_bytes) {
                names.push(format!("{} (={})", role, known_name));
            } else {
                names.push(role.clone());
                known_pubkeys.insert(pubkey_bytes, role.clone());
            }
        } else if let Some(known_name) = known_pubkeys.get(&pubkey_bytes) {
            // No role, but matches a known account
            names.push(format!("packed_{} (={})", packed_idx, known_name));
        } else {
            // Unknown packed account
            names.push(format!("packed_account_{}", packed_idx));
        }
        idx += 1;
        packed_idx = packed_idx.saturating_add(1);
    }

    names
}

/// Resolve MintAction account names dynamically based on instruction data.
///
/// MintAction has a dynamic account layout that depends on:
/// - `create_mint`: whether creating a new compressed mint
/// - `cpi_context`: whether using CPI context mode
/// - `mint` (None = decompressed): whether mint is decompressed to CMint
/// - `actions`: may contain DecompressMint, CompressAndCloseMint, MintToCompressed
///
/// Account layout (see plan for full details):
/// 1. Fixed: light_system_program, [mint_signer if create_mint], authority
/// 2. CPI Context Mode: fee_payer, cpi_authority_pda, cpi_context
/// 3. Executing Mode:
///    - Optional: compressible_config, cmint, rent_sponsor
///    - LightSystemAccounts (6 required)
///    - Optional: cpi_context_account
///    - Tree accounts
///    - Packed accounts (identified by pubkey when possible)
#[cfg(not(target_os = "solana"))]
pub fn resolve_mint_action_account_names(
    data: &MintActionCompressedInstructionData,
    accounts: &[AccountMeta],
) -> Vec<String> {
    use std::collections::HashMap;

    use light_token_interface::instructions::mint_action::Action;

    let mut names = Vec::with_capacity(accounts.len());
    let mut idx = 0;
    // Track known pubkeys -> name for identifying packed accounts
    let mut known_pubkeys: HashMap<[u8; 32], String> = HashMap::new();

    // Helper to add name and track pubkey
    let mut add_name = |name: &str,
                        accounts: &[AccountMeta],
                        idx: &mut usize,
                        known: &mut HashMap<[u8; 32], String>| {
        if *idx < accounts.len() {
            names.push(name.to_string());
            known.insert(accounts[*idx].pubkey.to_bytes(), name.to_string());
            *idx += 1;
            true
        } else {
            false
        }
    };

    // Index 0: light_system_program (always)
    add_name(
        "light_system_program",
        accounts,
        &mut idx,
        &mut known_pubkeys,
    );

    // Index 1: mint_signer (optional - only if creating mint)
    if data.create_mint.is_some() {
        add_name("mint_signer", accounts, &mut idx, &mut known_pubkeys);
    }

    // Next: authority (always)
    add_name("authority", accounts, &mut idx, &mut known_pubkeys);

    // Determine flags from instruction data
    let write_to_cpi_context = data
        .cpi_context
        .as_ref()
        .map(|ctx| ctx.first_set_context || ctx.set_context)
        .unwrap_or(false);

    if write_to_cpi_context {
        // CPI Context Mode: CpiContextLightSystemAccounts (3 accounts)
        add_name("fee_payer", accounts, &mut idx, &mut known_pubkeys);
        add_name("cpi_authority_pda", accounts, &mut idx, &mut known_pubkeys);
        add_name("cpi_context", accounts, &mut idx, &mut known_pubkeys);
        // No more accounts in this mode
    } else {
        // Executing Mode
        let has_decompress_mint_action = data
            .actions
            .iter()
            .any(|action| matches!(action, Action::DecompressMint(_)));

        let has_compress_and_close_cmint_action = data
            .actions
            .iter()
            .any(|action| matches!(action, Action::CompressAndCloseMint(_)));

        let needs_compressible_accounts =
            has_decompress_mint_action || has_compress_and_close_cmint_action;

        let cmint_decompressed = data.mint.is_none();
        let needs_cmint_account =
            cmint_decompressed || has_decompress_mint_action || has_compress_and_close_cmint_action;

        // Optional: compressible_config
        if needs_compressible_accounts {
            add_name(
                "compressible_config",
                accounts,
                &mut idx,
                &mut known_pubkeys,
            );
        }

        // Optional: cmint
        if needs_cmint_account {
            add_name("cmint", accounts, &mut idx, &mut known_pubkeys);
        }

        // Optional: rent_sponsor
        if needs_compressible_accounts {
            add_name("rent_sponsor", accounts, &mut idx, &mut known_pubkeys);
        }

        // LightSystemAccounts (6 required)
        add_name("fee_payer", accounts, &mut idx, &mut known_pubkeys);
        add_name("cpi_authority_pda", accounts, &mut idx, &mut known_pubkeys);
        add_name(
            "registered_program_pda",
            accounts,
            &mut idx,
            &mut known_pubkeys,
        );
        add_name(
            "account_compression_authority",
            accounts,
            &mut idx,
            &mut known_pubkeys,
        );
        add_name(
            "account_compression_program",
            accounts,
            &mut idx,
            &mut known_pubkeys,
        );
        add_name("system_program", accounts, &mut idx, &mut known_pubkeys);

        // Note: cpi_context_account and tree accounts are NOT named here -
        // let the formatter use the transaction-level account names
    }

    names
}

/// Format MintAction instruction data with resolved pubkeys.
///
/// Calculate the packed accounts start position for MintAction.
///
/// MintAction has a simpler layout than Transfer2:
/// - 6 fixed LightSystemAccounts: fee_payer, cpi_authority_pda, registered_program_pda,
///   account_compression_authority, account_compression_program, system_program
/// - Optional: cpi_context_account (when cpi_context is present but not writing)
/// - Then: packed accounts
#[cfg(not(target_os = "solana"))]
fn calculate_mint_action_packed_accounts_start(
    data: &MintActionCompressedInstructionData,
) -> usize {
    let cpi_context_write_mode = data
        .cpi_context
        .as_ref()
        .map(|ctx| ctx.set_context || ctx.first_set_context)
        .unwrap_or(false);

    if cpi_context_write_mode {
        // CPI context write mode: [fee_payer, cpi_authority_pda, cpi_context]
        3
    } else {
        // Normal mode: 6 LightSystemAccounts + optional cpi_context_account
        let mut start = 6;
        if data.cpi_context.is_some() {
            start += 1; // cpi_context_account
        }
        start
    }
}

/// Format MintAction instruction data with resolved pubkeys.
///
/// This formatter provides a human-readable view of the mint action instruction,
/// resolving account indices to actual pubkeys from the instruction accounts.
///
/// Mode detection:
/// - CPI context write mode (cpi_context.set_context || first_set_context): Shows raw indices
/// - Direct mode: Resolves packed account indices using dynamically calculated start position
#[cfg(not(target_os = "solana"))]
pub fn format_mint_action(
    data: &MintActionCompressedInstructionData,
    accounts: &[AccountMeta],
) -> String {
    use std::fmt::Write;

    use light_token_interface::instructions::mint_action::Action;
    let mut output = String::new();

    // CPI context write mode: set_context OR first_set_context means packed accounts in CPI context
    let cpi_context_write_mode = data
        .cpi_context
        .as_ref()
        .map(|ctx| ctx.set_context || ctx.first_set_context)
        .unwrap_or(false);

    // Calculate where packed accounts start based on instruction configuration
    let packed_accounts_start = calculate_mint_action_packed_accounts_start(data);

    // Helper to resolve account index
    let resolve = |index: u8| -> String {
        if cpi_context_write_mode {
            format!("packed[{}]", index)
        } else {
            accounts
                .get(packed_accounts_start + index as usize)
                .map(|a| a.pubkey.to_string())
                .unwrap_or_else(|| format!("OUT_OF_BOUNDS({})", index))
        }
    };

    // Header with mode indicator
    if cpi_context_write_mode {
        let _ = writeln!(
            output,
            "[CPI Context Write Mode - packed accounts in CPI context]"
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
    if data.max_top_up != u16::MAX {
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
    /// Uses dynamic account names resolver because the account layout depends on instruction data.
    #[discriminator = 101]
    #[instruction_decoder(
        params = CompressedTokenInstructionDataTransfer2,
        account_names_resolver_from_params = crate::programs::ctoken::resolve_transfer2_account_names,
        pretty_formatter = crate::programs::ctoken::format_transfer2
    )]
    Transfer2,

    /// Create associated token account idempotently (discriminator 102)
    #[discriminator = 102]
    #[instruction_decoder(account_names = ["owner", "mint", "fee_payer", "ata", "system_program", "config", "rent_payer"])]
    CreateAssociatedTokenAccountIdempotent,

    /// Mint action for compressed tokens (discriminator 103)
    /// Uses dynamic account names resolver because the account layout depends on instruction data.
    #[discriminator = 103]
    #[instruction_decoder(
        params = MintActionCompressedInstructionData,
        account_names_resolver_from_params = crate::programs::ctoken::resolve_mint_action_account_names,
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
