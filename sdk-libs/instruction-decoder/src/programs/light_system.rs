//! Light System Program instruction decoder.
//!
//! This module provides a macro-derived decoder for the Light System Program,
//! which uses 8-byte discriminators for compressed account operations.
//!
//! ## Instructions
//!
//! - `Invoke`: Direct invocation of Light System (has 4-byte Anchor prefix after discriminator)
//! - `InvokeCpi`: CPI invocation from another program (has 4-byte Anchor prefix after discriminator)
//! - `InvokeCpiWithReadOnly`: CPI with read-only accounts (no prefix, borsh-only)
//! - `InvokeCpiWithAccountInfo`: CPI with full account info (no prefix, borsh-only)

// Allow the macro-generated code to reference types from this crate
extern crate self as light_instruction_decoder;

use borsh::BorshDeserialize;
use light_compressed_account::instruction_data::{
    data::InstructionDataInvoke, invoke_cpi::InstructionDataInvokeCpi,
    with_account_info::InstructionDataInvokeCpiWithAccountInfo,
    with_readonly::InstructionDataInvokeCpiWithReadOnly,
};
use light_instruction_decoder_derive::InstructionDecoder;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

/// System program ID string for account resolution
const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";

// ============================================================================
// Helper Functions for Deduplicating Formatter Code
// ============================================================================

/// Format input compressed accounts section for Invoke/InvokeCpi.
///
/// Formats `PackedCompressedAccountWithMerkleContext` accounts with:
/// owner, address, lamports, data_hash, discriminator, merkle_tree, leaf_index, root_index
#[cfg(not(target_os = "solana"))]
fn format_input_accounts_section(
    output: &mut String,
    accounts: &[light_compressed_account::compressed_account::PackedCompressedAccountWithMerkleContext],
    instruction_accounts: &[AccountMeta],
) {
    use std::fmt::Write;

    if accounts.is_empty() {
        return;
    }

    let _ = writeln!(output, "Input Accounts ({}):", accounts.len());
    for (i, acc) in accounts.iter().enumerate() {
        let _ = writeln!(output, "  [{}]", i);
        let _ = writeln!(
            output,
            "      owner: {}",
            Pubkey::new_from_array(acc.compressed_account.owner.to_bytes())
        );
        if let Some(addr) = acc.compressed_account.address {
            let _ = writeln!(output, "      address: {:?}", addr);
        }
        let _ = writeln!(
            output,
            "      lamports: {}",
            acc.compressed_account.lamports
        );
        if let Some(ref acc_data) = acc.compressed_account.data {
            let _ = writeln!(output, "      data_hash: {:?}", acc_data.data_hash);
            let _ = writeln!(output, "      discriminator: {:?}", acc_data.discriminator);
        }
        let tree_idx = Some(acc.merkle_context.merkle_tree_pubkey_index);
        let queue_idx = Some(acc.merkle_context.queue_pubkey_index);
        let (tree_pubkey, _queue_pubkey) =
            resolve_tree_and_queue_pubkeys(instruction_accounts, tree_idx, queue_idx);
        if let Some(tp) = tree_pubkey {
            let _ = writeln!(
                output,
                "      merkle_tree_pubkey (index {}): {}",
                acc.merkle_context.merkle_tree_pubkey_index, tp
            );
        }
        let _ = writeln!(
            output,
            "      leaf_index: {}",
            acc.merkle_context.leaf_index
        );
        let _ = writeln!(output, "      root_index: {}", acc.root_index);
    }
}

/// Format input compressed accounts section for InvokeCpiWithReadOnly.
///
/// Formats `InAccount` accounts with a shared owner from `invoking_program_id`.
#[cfg(not(target_os = "solana"))]
fn format_readonly_input_accounts_section(
    output: &mut String,
    accounts: &[light_compressed_account::instruction_data::with_readonly::InAccount],
    invoking_program_id: &light_compressed_account::pubkey::Pubkey,
    instruction_accounts: &[AccountMeta],
) {
    use std::fmt::Write;

    if accounts.is_empty() {
        return;
    }

    let _ = writeln!(output, "Input Accounts ({}):", accounts.len());
    for (i, acc) in accounts.iter().enumerate() {
        let _ = writeln!(output, "  [{}]", i);
        let _ = writeln!(
            output,
            "      owner: {}",
            Pubkey::new_from_array(invoking_program_id.to_bytes())
        );
        if let Some(addr) = acc.address {
            let _ = writeln!(output, "      address: {:?}", addr);
        }
        let _ = writeln!(output, "      lamports: {}", acc.lamports);
        let _ = writeln!(output, "      data_hash: {:?}", acc.data_hash);
        let _ = writeln!(output, "      discriminator: {:?}", acc.discriminator);
        let tree_idx = Some(acc.merkle_context.merkle_tree_pubkey_index);
        let queue_idx = Some(acc.merkle_context.queue_pubkey_index);
        let (tree_pubkey, _queue_pubkey) =
            resolve_tree_and_queue_pubkeys(instruction_accounts, tree_idx, queue_idx);
        if let Some(tp) = tree_pubkey {
            let _ = writeln!(
                output,
                "      merkle_tree_pubkey (index {}): {}",
                acc.merkle_context.merkle_tree_pubkey_index, tp
            );
        }
        let _ = writeln!(
            output,
            "      leaf_index: {}",
            acc.merkle_context.leaf_index
        );
        let _ = writeln!(output, "      root_index: {}", acc.root_index);
    }
}

/// Format output compressed accounts section.
///
/// Formats `OutputCompressedAccountWithPackedContext` accounts with:
/// owner, address, lamports, data_hash, discriminator, data, merkle_tree
#[cfg(not(target_os = "solana"))]
fn format_output_accounts_section(
    output: &mut String,
    accounts: &[light_compressed_account::instruction_data::data::OutputCompressedAccountWithPackedContext],
    instruction_accounts: &[AccountMeta],
) {
    use std::fmt::Write;

    if accounts.is_empty() {
        return;
    }

    let _ = writeln!(output, "Output Accounts ({}):", accounts.len());
    for (i, acc) in accounts.iter().enumerate() {
        let _ = writeln!(output, "  [{}]", i);
        let _ = writeln!(
            output,
            "      owner: {}",
            Pubkey::new_from_array(acc.compressed_account.owner.to_bytes())
        );
        if let Some(addr) = acc.compressed_account.address {
            let _ = writeln!(output, "      address: {:?}", addr);
        }
        let _ = writeln!(
            output,
            "      lamports: {}",
            acc.compressed_account.lamports
        );
        if let Some(ref acc_data) = acc.compressed_account.data {
            let _ = writeln!(output, "      data_hash: {:?}", acc_data.data_hash);
            let _ = writeln!(output, "      discriminator: {:?}", acc_data.discriminator);
            let _ = writeln!(
                output,
                "      data ({} bytes): {:?}",
                acc_data.data.len(),
                acc_data.data
            );
        }
        let tree_idx = Some(acc.merkle_tree_index);
        let (tree_pubkey, _) = resolve_tree_and_queue_pubkeys(instruction_accounts, tree_idx, None);
        if let Some(tp) = tree_pubkey {
            let _ = writeln!(
                output,
                "      merkle_tree_pubkey (index {}): {}",
                acc.merkle_tree_index, tp
            );
        }
    }
}

/// Format output compressed accounts section for InvokeCpiWithReadOnly.
///
/// Uses `invoking_program_id` as owner instead of per-account owner.
#[cfg(not(target_os = "solana"))]
fn format_readonly_output_accounts_section(
    output: &mut String,
    accounts: &[light_compressed_account::instruction_data::data::OutputCompressedAccountWithPackedContext],
    invoking_program_id: &light_compressed_account::pubkey::Pubkey,
    instruction_accounts: &[AccountMeta],
) {
    use std::fmt::Write;

    if accounts.is_empty() {
        return;
    }

    let _ = writeln!(output, "Output Accounts ({}):", accounts.len());
    for (i, acc) in accounts.iter().enumerate() {
        let _ = writeln!(output, "  [{}]", i);
        let _ = writeln!(
            output,
            "      owner: {}",
            Pubkey::new_from_array(invoking_program_id.to_bytes())
        );
        if let Some(addr) = acc.compressed_account.address {
            let _ = writeln!(output, "      address: {:?}", addr);
        }
        let _ = writeln!(
            output,
            "      lamports: {}",
            acc.compressed_account.lamports
        );
        if let Some(ref acc_data) = acc.compressed_account.data {
            let _ = writeln!(output, "      data_hash: {:?}", acc_data.data_hash);
            let _ = writeln!(output, "      discriminator: {:?}", acc_data.discriminator);
            let _ = writeln!(
                output,
                "      data ({} bytes): {:?}",
                acc_data.data.len(),
                acc_data.data
            );
        }
        let tree_idx = Some(acc.merkle_tree_index);
        let (tree_pubkey, _) = resolve_tree_and_queue_pubkeys(instruction_accounts, tree_idx, None);
        if let Some(tp) = tree_pubkey {
            let _ = writeln!(
                output,
                "      merkle_tree_pubkey (index {}): {}",
                acc.merkle_tree_index, tp
            );
        }
    }
}

/// Format new address params section for Invoke/InvokeCpi.
///
/// Formats `NewAddressParamsPacked` with: seed, queue, tree
#[cfg(not(target_os = "solana"))]
fn format_new_address_params_section(
    output: &mut String,
    params: &[light_compressed_account::instruction_data::data::NewAddressParamsPacked],
    instruction_accounts: &[AccountMeta],
) {
    use std::fmt::Write;

    if params.is_empty() {
        return;
    }

    let _ = writeln!(output, "New Addresses ({}):", params.len());
    for (i, param) in params.iter().enumerate() {
        let _ = writeln!(output, "  [{}] seed: {:?}", i, param.seed);
        let tree_idx = Some(param.address_merkle_tree_account_index);
        let queue_idx = Some(param.address_queue_account_index);
        let (tree_pubkey, queue_pubkey) =
            resolve_tree_and_queue_pubkeys(instruction_accounts, tree_idx, queue_idx);
        if let Some(qp) = queue_pubkey {
            let _ = writeln!(
                output,
                "      queue[{}]: {}",
                param.address_queue_account_index, qp
            );
        }
        if let Some(tp) = tree_pubkey {
            let _ = writeln!(
                output,
                "      tree[{}]: {}",
                param.address_merkle_tree_account_index, tp
            );
        }
    }
}

/// Format new address params section with assignment info.
///
/// Formats `NewAddressParamsAssignedPacked` with: seed, queue, tree, assigned
#[cfg(not(target_os = "solana"))]
fn format_new_address_params_assigned_section(
    output: &mut String,
    params: &[light_compressed_account::instruction_data::data::NewAddressParamsAssignedPacked],
    instruction_accounts: &[AccountMeta],
) {
    use std::fmt::Write;

    if params.is_empty() {
        return;
    }

    let _ = writeln!(output, "New Addresses ({}):", params.len());
    for (i, param) in params.iter().enumerate() {
        let _ = writeln!(output, "  [{}] seed: {:?}", i, param.seed);
        let tree_idx = Some(param.address_merkle_tree_account_index);
        let queue_idx = Some(param.address_queue_account_index);
        let (tree_pubkey, queue_pubkey) =
            resolve_tree_and_queue_pubkeys(instruction_accounts, tree_idx, queue_idx);
        if let Some(qp) = queue_pubkey {
            let _ = writeln!(
                output,
                "      queue[{}]: {}",
                param.address_queue_account_index, qp
            );
        }
        if let Some(tp) = tree_pubkey {
            let _ = writeln!(
                output,
                "      tree[{}]: {}",
                param.address_merkle_tree_account_index, tp
            );
        }
        let assigned = if param.assigned_to_account {
            format!("account[{}]", param.assigned_account_index)
        } else {
            "None".to_string()
        };
        let _ = writeln!(output, "      assigned: {}", assigned);
    }
}

/// Format read-only addresses section.
///
/// Formats `PackedReadOnlyAddress` with: address, tree
#[cfg(not(target_os = "solana"))]
fn format_read_only_addresses_section(
    output: &mut String,
    addresses: &[light_compressed_account::instruction_data::data::PackedReadOnlyAddress],
    instruction_accounts: &[AccountMeta],
) {
    use std::fmt::Write;

    if addresses.is_empty() {
        return;
    }

    let _ = writeln!(output, "Read-Only Addresses ({}):", addresses.len());
    for (i, addr) in addresses.iter().enumerate() {
        let _ = writeln!(output, "  [{}] address: {:?}", i, addr.address);
        let tree_idx = Some(addr.address_merkle_tree_account_index);
        let (tree_pubkey, _) = resolve_tree_and_queue_pubkeys(instruction_accounts, tree_idx, None);
        if let Some(tp) = tree_pubkey {
            let _ = writeln!(
                output,
                "      tree[{}]: {}",
                addr.address_merkle_tree_account_index, tp
            );
        }
    }
}

/// Format compress/decompress and relay fee section for Invoke/InvokeCpi.
#[cfg(not(target_os = "solana"))]
fn format_fee_section(
    output: &mut String,
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
    relay_fee: Option<u64>,
) {
    use std::fmt::Write;

    if let Some(lamports) = compress_or_decompress_lamports {
        let _ = writeln!(
            output,
            "Compress/Decompress: {} lamports (is_compress: {})",
            lamports, is_compress
        );
    }

    if let Some(fee) = relay_fee {
        let _ = writeln!(output, "Relay fee: {} lamports", fee);
    }
}

/// Format compress/decompress section for ReadOnly/AccountInfo variants.
///
/// Uses u64 directly instead of Option<u64>.
#[cfg(not(target_os = "solana"))]
fn format_compress_decompress_section(
    output: &mut String,
    compress_or_decompress_lamports: u64,
    is_compress: bool,
) {
    use std::fmt::Write;

    if compress_or_decompress_lamports > 0 {
        let _ = writeln!(
            output,
            "Compress/Decompress: {} lamports (is_compress: {})",
            compress_or_decompress_lamports, is_compress
        );
    }
}

/// Format account infos section for InvokeCpiWithAccountInfo.
///
/// Formats `CompressedAccountInfo` with combined input/output per account.
#[cfg(not(target_os = "solana"))]
fn format_account_infos_section(
    output: &mut String,
    account_infos: &[light_compressed_account::instruction_data::with_account_info::CompressedAccountInfo],
    instruction_accounts: &[AccountMeta],
) {
    use std::fmt::Write;

    if account_infos.is_empty() {
        return;
    }

    let _ = writeln!(output, "Account Infos ({}):", account_infos.len());
    for (i, account_info) in account_infos.iter().enumerate() {
        let _ = writeln!(output, "  [{}]", i);
        if let Some(addr) = account_info.address {
            let _ = writeln!(output, "      address: {:?}", addr);
        }

        if let Some(ref input) = account_info.input {
            let _ = writeln!(output, "    Input:");
            let _ = writeln!(output, "      lamports: {}", input.lamports);
            let _ = writeln!(output, "      data_hash: {:?}", input.data_hash);
            let _ = writeln!(output, "      discriminator: {:?}", input.discriminator);
            let _ = writeln!(
                output,
                "      leaf_index: {}",
                input.merkle_context.leaf_index
            );
            let _ = writeln!(output, "      root_index: {}", input.root_index);
        }

        if let Some(ref out) = account_info.output {
            let _ = writeln!(output, "    Output:");
            let _ = writeln!(output, "      lamports: {}", out.lamports);
            let _ = writeln!(output, "      data_hash: {:?}", out.data_hash);
            let _ = writeln!(output, "      discriminator: {:?}", out.discriminator);
            if !out.data.is_empty() {
                let _ = writeln!(
                    output,
                    "      data ({} bytes): {:?}",
                    out.data.len(),
                    out.data
                );
            }
            let tree_idx = Some(out.output_merkle_tree_index);
            let (tree_pubkey, _) =
                resolve_tree_and_queue_pubkeys(instruction_accounts, tree_idx, None);
            if let Some(tp) = tree_pubkey {
                let _ = writeln!(
                    output,
                    "      merkle_tree_pubkey (index {}): {}",
                    out.output_merkle_tree_index, tp
                );
            }
        }
    }
}

/// Helper to resolve merkle tree and queue pubkeys from instruction accounts.
/// Tree accounts start 2 positions after the system program account.
fn resolve_tree_and_queue_pubkeys(
    accounts: &[AccountMeta],
    merkle_tree_index: Option<u8>,
    nullifier_queue_index: Option<u8>,
) -> (Option<Pubkey>, Option<Pubkey>) {
    let mut tree_pubkey = None;
    let mut queue_pubkey = None;

    // Find the system program account position
    let mut system_program_pos = None;
    for (i, account) in accounts.iter().enumerate() {
        if account.pubkey.to_string() == SYSTEM_PROGRAM_ID {
            system_program_pos = Some(i);
            break;
        }
    }

    if let Some(system_pos) = system_program_pos {
        // Tree accounts start 2 positions after system program
        let tree_accounts_start = system_pos + 2;

        if let Some(tree_idx) = merkle_tree_index {
            let tree_account_pos = tree_accounts_start + tree_idx as usize;
            if tree_account_pos < accounts.len() {
                tree_pubkey = Some(accounts[tree_account_pos].pubkey);
            }
        }

        if let Some(queue_idx) = nullifier_queue_index {
            let queue_account_pos = tree_accounts_start + queue_idx as usize;
            if queue_account_pos < accounts.len() {
                queue_pubkey = Some(accounts[queue_account_pos].pubkey);
            }
        }
    }

    (tree_pubkey, queue_pubkey)
}

/// Format InvokeCpiWithReadOnly instruction data.
///
/// Note: This instruction does NOT have the 4-byte Anchor prefix - it uses pure borsh.
#[cfg(not(target_os = "solana"))]
pub fn format_invoke_cpi_readonly(
    data: &InstructionDataInvokeCpiWithReadOnly,
    accounts: &[AccountMeta],
) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    let _ = writeln!(
        output,
        "Accounts: in: {}, out: {}",
        data.input_compressed_accounts.len(),
        data.output_compressed_accounts.len()
    );
    let _ = writeln!(output, "Proof: Validity proof");

    format_readonly_input_accounts_section(
        &mut output,
        &data.input_compressed_accounts,
        &data.invoking_program_id,
        accounts,
    );
    format_readonly_output_accounts_section(
        &mut output,
        &data.output_compressed_accounts,
        &data.invoking_program_id,
        accounts,
    );
    format_new_address_params_assigned_section(&mut output, &data.new_address_params, accounts);
    format_read_only_addresses_section(&mut output, &data.read_only_addresses, accounts);
    format_compress_decompress_section(
        &mut output,
        data.compress_or_decompress_lamports,
        data.is_compress,
    );

    output
}

/// Resolve account names dynamically for InvokeCpiWithReadOnly.
///
/// Account layout depends on CPI context mode:
///
/// **CPI Context Write Mode** (`set_context || first_set_context`):
/// - fee_payer, cpi_authority_pda, cpi_context
///
/// **Normal Mode**:
/// 1. Fixed: fee_payer, authority, registered_program_pda, account_compression_authority,
///    account_compression_program, system_program
/// 2. Optional: cpi_context_account (if cpi_context is present)
/// 3. Tree accounts: named based on usage in instruction data
#[cfg(not(target_os = "solana"))]
pub fn resolve_invoke_cpi_readonly_account_names(
    data: &InstructionDataInvokeCpiWithReadOnly,
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

    // Check if we're in CPI context write mode
    let cpi_context_write_mode = data.cpi_context.set_context || data.cpi_context.first_set_context;

    if cpi_context_write_mode {
        // CPI Context Write Mode: only 3 accounts
        add_name("fee_payer", accounts, &mut idx, &mut known_pubkeys);
        add_name("cpi_authority_pda", accounts, &mut idx, &mut known_pubkeys);
        add_name("cpi_context", accounts, &mut idx, &mut known_pubkeys);
        return names;
    }

    // Normal Mode: Fixed LightSystemAccounts (6 accounts)
    add_name("fee_payer", accounts, &mut idx, &mut known_pubkeys);
    add_name("authority", accounts, &mut idx, &mut known_pubkeys);
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

    // Don't provide names for remaining accounts (cpi_context_account, tree/queue accounts)
    // - let the formatter use the transaction-level account names
    names
}

/// Format InvokeCpiWithAccountInfo instruction data.
///
/// Note: This instruction does NOT have the 4-byte Anchor prefix - it uses pure borsh.
#[cfg(not(target_os = "solana"))]
pub fn format_invoke_cpi_account_info(
    data: &InstructionDataInvokeCpiWithAccountInfo,
    accounts: &[AccountMeta],
) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    let input_count = data
        .account_infos
        .iter()
        .filter(|a| a.input.is_some())
        .count();
    let output_count = data
        .account_infos
        .iter()
        .filter(|a| a.output.is_some())
        .count();

    let _ = writeln!(
        output,
        "Accounts: in: {}, out: {}",
        input_count, output_count
    );
    let _ = writeln!(output, "Proof: Validity proof");

    // Account infos with input/output (unique structure, kept inline)
    format_account_infos_section(&mut output, &data.account_infos, accounts);

    format_new_address_params_assigned_section(&mut output, &data.new_address_params, accounts);
    format_read_only_addresses_section(&mut output, &data.read_only_addresses, accounts);
    format_compress_decompress_section(
        &mut output,
        data.compress_or_decompress_lamports,
        data.is_compress,
    );

    output
}

/// Resolve account names dynamically for InvokeCpiWithAccountInfo.
///
/// Account layout depends on CPI context mode:
///
/// **CPI Context Write Mode** (`set_context || first_set_context`):
/// - fee_payer, cpi_authority_pda, cpi_context
///
/// **Normal Mode**:
/// 1. Fixed: fee_payer, authority, registered_program_pda, account_compression_authority,
///    account_compression_program, system_program
/// 2. Optional: cpi_context_account (if cpi_context is present)
/// 3. Tree accounts: named based on usage in instruction data
#[cfg(not(target_os = "solana"))]
pub fn resolve_invoke_cpi_account_info_account_names(
    data: &InstructionDataInvokeCpiWithAccountInfo,
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

    // Check if we're in CPI context write mode
    let cpi_context_write_mode = data.cpi_context.set_context || data.cpi_context.first_set_context;

    if cpi_context_write_mode {
        // CPI Context Write Mode: only 3 accounts
        add_name("fee_payer", accounts, &mut idx, &mut known_pubkeys);
        add_name("cpi_authority_pda", accounts, &mut idx, &mut known_pubkeys);
        add_name("cpi_context", accounts, &mut idx, &mut known_pubkeys);
        return names;
    }

    // Normal Mode: Fixed LightSystemAccounts (6 accounts)
    add_name("fee_payer", accounts, &mut idx, &mut known_pubkeys);
    add_name("authority", accounts, &mut idx, &mut known_pubkeys);
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

    // Don't provide names for remaining accounts (cpi_context_account, tree/queue accounts)
    // - let the formatter use the transaction-level account names
    names
}

/// Wrapper type for Invoke instruction that handles the 4-byte Anchor prefix.
///
/// The derive macro's borsh deserialization expects the data immediately after
/// the discriminator, but Invoke/InvokeCpi have a 4-byte vec length prefix.
/// This wrapper type's deserialize implementation skips those 4 bytes.
#[derive(Debug)]
pub struct InvokeWrapper(pub InstructionDataInvoke);

impl BorshDeserialize for InvokeWrapper {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        // Skip 4-byte Anchor vec length prefix
        let mut prefix = [0u8; 4];
        reader.read_exact(&mut prefix)?;
        // Deserialize the actual data
        let inner = InstructionDataInvoke::deserialize_reader(reader)?;
        Ok(InvokeWrapper(inner))
    }
}

/// Wrapper type for InvokeCpi instruction that handles the 4-byte Anchor prefix.
#[derive(Debug)]
pub struct InvokeCpiWrapper(pub InstructionDataInvokeCpi);

impl BorshDeserialize for InvokeCpiWrapper {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        // Skip 4-byte Anchor vec length prefix
        let mut prefix = [0u8; 4];
        reader.read_exact(&mut prefix)?;
        // Deserialize the actual data
        let inner = InstructionDataInvokeCpi::deserialize_reader(reader)?;
        Ok(InvokeCpiWrapper(inner))
    }
}

/// Formatter wrapper that takes raw bytes and handles the prefix skip internally.
#[cfg(not(target_os = "solana"))]
pub fn format_invoke_wrapper(data: &InvokeWrapper, accounts: &[AccountMeta]) -> String {
    // We already have the parsed data, format it directly
    format_invoke_inner(&data.0, accounts)
}

/// Formatter wrapper that takes raw bytes and handles the prefix skip internally.
#[cfg(not(target_os = "solana"))]
pub fn format_invoke_cpi_wrapper(data: &InvokeCpiWrapper, accounts: &[AccountMeta]) -> String {
    // We already have the parsed data, format it directly
    format_invoke_cpi_inner(&data.0, accounts)
}

/// Format InstructionDataInvoke (internal helper).
#[cfg(not(target_os = "solana"))]
fn format_invoke_inner(data: &InstructionDataInvoke, accounts: &[AccountMeta]) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    let _ = writeln!(
        output,
        "Accounts: in: {}, out: {}",
        data.input_compressed_accounts_with_merkle_context.len(),
        data.output_compressed_accounts.len()
    );

    if data.proof.is_some() {
        let _ = writeln!(output, "Proof: Validity proof");
    }

    format_input_accounts_section(
        &mut output,
        &data.input_compressed_accounts_with_merkle_context,
        accounts,
    );
    format_output_accounts_section(&mut output, &data.output_compressed_accounts, accounts);
    format_new_address_params_section(&mut output, &data.new_address_params, accounts);
    format_fee_section(
        &mut output,
        data.compress_or_decompress_lamports,
        data.is_compress,
        data.relay_fee,
    );

    output
}

/// Format InstructionDataInvokeCpi (internal helper).
#[cfg(not(target_os = "solana"))]
fn format_invoke_cpi_inner(data: &InstructionDataInvokeCpi, accounts: &[AccountMeta]) -> String {
    use std::fmt::Write;
    let mut output = String::new();

    let _ = writeln!(
        output,
        "Accounts: in: {}, out: {}",
        data.input_compressed_accounts_with_merkle_context.len(),
        data.output_compressed_accounts.len()
    );

    if data.proof.is_some() {
        let _ = writeln!(output, "Proof: Validity proof");
    }

    format_input_accounts_section(
        &mut output,
        &data.input_compressed_accounts_with_merkle_context,
        accounts,
    );
    format_output_accounts_section(&mut output, &data.output_compressed_accounts, accounts);
    format_new_address_params_section(&mut output, &data.new_address_params, accounts);
    format_fee_section(
        &mut output,
        data.compress_or_decompress_lamports,
        data.is_compress,
        data.relay_fee,
    );

    output
}

/// Light System Program instructions.
///
/// The Light System Program uses 8-byte discriminators for compressed account operations.
/// Each instruction has an explicit discriminator attribute.
#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7",
    program_name = "Light System Program",
    discriminator_size = 8
)]
pub enum LightSystemInstruction {
    /// Direct invocation of Light System - creates/modifies compressed accounts.
    /// Has 4-byte Anchor vec length prefix after discriminator.
    #[discriminator(26, 16, 169, 7, 21, 202, 242, 25)]
    #[instruction_decoder(
        account_names = ["fee_payer", "authority", "registered_program_pda", "log_program", "account_compression_authority", "account_compression_program", "self_program"],
        params = InvokeWrapper,
        pretty_formatter = crate::programs::light_system::format_invoke_wrapper
    )]
    Invoke,

    /// CPI invocation from another program.
    /// Has 4-byte Anchor vec length prefix after discriminator.
    #[discriminator(49, 212, 191, 129, 39, 194, 43, 196)]
    #[instruction_decoder(
        account_names = ["fee_payer", "authority", "registered_program_pda", "log_program", "account_compression_authority", "account_compression_program", "invoking_program", "cpi_signer"],
        params = InvokeCpiWrapper,
        pretty_formatter = crate::programs::light_system::format_invoke_cpi_wrapper
    )]
    InvokeCpi,

    /// CPI with read-only compressed accounts (V2 account layout).
    /// Uses pure borsh serialization (no 4-byte prefix).
    /// Note: V2 instructions have no log_program account.
    #[discriminator(86, 47, 163, 166, 21, 223, 92, 8)]
    #[instruction_decoder(
        params = InstructionDataInvokeCpiWithReadOnly,
        account_names_resolver_from_params = crate::programs::light_system::resolve_invoke_cpi_readonly_account_names,
        pretty_formatter = crate::programs::light_system::format_invoke_cpi_readonly
    )]
    InvokeCpiWithReadOnly,

    /// CPI with full account info for each compressed account (V2 account layout).
    /// Uses pure borsh serialization (no 4-byte prefix).
    /// Note: V2 instructions have no log_program account.
    #[discriminator(228, 34, 128, 84, 47, 139, 86, 240)]
    #[instruction_decoder(
        params = InstructionDataInvokeCpiWithAccountInfo,
        account_names_resolver_from_params = crate::programs::light_system::resolve_invoke_cpi_account_info_account_names,
        pretty_formatter = crate::programs::light_system::format_invoke_cpi_account_info
    )]
    InvokeCpiWithAccountInfo,
}
