//! LiteSVM integration for Light Protocol logging
//!
//! This module provides the glue layer between LiteSVM and the instruction-decoder crate.
//! All logging types, decoders, and formatting utilities are in `light-instruction-decoder`.
//!
//! This module only contains:
//! - LiteSVM-specific transaction result parsing (`from_transaction_result`)
//! - Log file I/O functions
//! - Re-exports from instruction-decoder

use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono;
// Re-export everything from instruction-decoder
pub use light_instruction_decoder::{
    AccountAccess, AccountChange, AccountCompressionInstructionDecoder, AccountStateSnapshot,
    CTokenInstructionDecoder, Colors, CompressedAccountInfo, ComputeBudgetInstructionDecoder,
    DecodedField, DecodedInstruction, DecoderRegistry, EnhancedInstructionLog,
    EnhancedLoggingConfig, EnhancedTransactionLog, InstructionDecoder, LightProtocolEvent,
    LightSystemInstructionDecoder, LogVerbosity, MerkleTreeChange, RegistryInstructionDecoder,
    SplTokenInstructionDecoder, SystemInstructionDecoder, Token2022InstructionDecoder,
    TransactionFormatter, TransactionStatus,
};
use litesvm::{types::TransactionResult, LiteSVM};
use solana_sdk::{
    inner_instruction::InnerInstruction, pubkey::Pubkey, signature::Signature,
    transaction::Transaction,
};

/// Lightweight pre-transaction account state capture.
/// Maps pubkey -> (lamports, data_len) for accounts in a transaction.
pub type AccountStates = HashMap<Pubkey, (u64, usize)>;

/// Capture account states from LiteSVM context.
/// Call this before and after sending the transaction.
pub fn capture_account_states(context: &LiteSVM, transaction: &Transaction) -> AccountStates {
    let mut states = HashMap::new();
    for pubkey in &transaction.message.account_keys {
        if let Some(account) = context.get_account(pubkey) {
            states.insert(*pubkey, (account.lamports, account.data.len()));
        } else {
            states.insert(*pubkey, (0, 0));
        }
    }
    states
}

use crate::program_test::config::ProgramTestConfig;

static SESSION_STARTED: std::sync::Once = std::sync::Once::new();

/// Get the log file path in target directory
fn get_log_file_path() -> PathBuf {
    use std::process::Command;
    if let Ok(output) = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version=1")
        .arg("--no-deps")
        .output()
    {
        if output.status.success() {
            if let Ok(metadata) = String::from_utf8(output.stdout) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&metadata) {
                    if let Some(target_directory) = json["target_directory"].as_str() {
                        let mut path = PathBuf::from(target_directory);
                        path.push("light_program_test.log");
                        return path;
                    }
                }
            }
        }
    }

    let mut path = PathBuf::from("target");
    path.push("light_program_test.log");
    path
}

/// Initialize log file with session header (called only once per session)
fn initialize_log_file() {
    SESSION_STARTED.call_once(|| {
        let log_path = get_log_file_path();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)
        {
            let datetime =
                chrono::DateTime::from_timestamp(timestamp as i64, 0).unwrap_or(chrono::Utc::now());
            let formatted_date = datetime.format("%Y-%m-%d %H:%M:%S UTC");

            let _ = writeln!(
                file,
                "=== Light Program Test Session Started at {} ===\n",
                formatted_date
            );
        }
    });
}

/// Strip ANSI escape codes from string for plain text log files
fn strip_ansi_codes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            for next_ch in chars.by_ref() {
                if next_ch == 'm' {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Write log entry to file (append to existing session log)
fn write_to_log_file(content: &str) {
    initialize_log_file();

    let log_path = get_log_file_path();

    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let clean_content = strip_ansi_codes(content);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(file, "{}", clean_content);
    }
}

/// Main entry point for enhanced transaction logging
#[allow(clippy::too_many_arguments)]
pub fn log_transaction_enhanced(
    config: &ProgramTestConfig,
    transaction: &Transaction,
    result: &TransactionResult,
    signature: &Signature,
    slot: u64,
    transaction_counter: usize,
    pre_states: Option<&AccountStates>,
    post_states: Option<&AccountStates>,
) {
    log_transaction_enhanced_with_console(
        config,
        transaction,
        result,
        signature,
        slot,
        transaction_counter,
        false,
        pre_states,
        post_states,
    )
}

/// Enhanced transaction logging with console output control
#[allow(clippy::too_many_arguments)]
pub fn log_transaction_enhanced_with_console(
    config: &ProgramTestConfig,
    transaction: &Transaction,
    result: &TransactionResult,
    signature: &Signature,
    slot: u64,
    transaction_counter: usize,
    print_to_console: bool,
    pre_states: Option<&AccountStates>,
    post_states: Option<&AccountStates>,
) {
    if !config.enhanced_logging.enabled {
        return;
    }

    let enhanced_log = from_transaction_result(
        transaction,
        result,
        signature,
        slot,
        &config.enhanced_logging,
        pre_states,
        post_states,
    );

    let formatter = TransactionFormatter::new(&config.enhanced_logging);
    let formatted_log = formatter.format(&enhanced_log, transaction_counter);

    write_to_log_file(&formatted_log);

    if print_to_console {
        println!("{}", formatted_log);
    }
}

/// Check if enhanced logging should be used instead of basic logging
pub fn should_use_enhanced_logging(config: &ProgramTestConfig) -> bool {
    config.enhanced_logging.enabled && !config.no_logs
}

// ============================================================================
// LiteSVM-specific conversion functions
// ============================================================================

/// Get human-readable program name from pubkey
fn get_program_name(program_id: &Pubkey) -> String {
    light_instruction_decoder::types::get_program_name(
        &solana_pubkey::Pubkey::new_from_array(program_id.to_bytes()),
        None,
    )
}

/// Use LiteSVM's pretty logs instead of parsing raw logs
fn get_pretty_logs_string(result: &TransactionResult) -> String {
    match result {
        Ok(meta) => meta.pretty_logs(),
        Err(failed) => failed.meta.pretty_logs(),
    }
}

/// Create EnhancedTransactionLog from LiteSVM transaction result
///
/// If pre_states and post_states are provided, captures account state snapshots
/// for all accounts in the transaction.
///
/// Use `capture_pre_account_states` before and after sending the transaction.
pub fn from_transaction_result(
    transaction: &Transaction,
    result: &TransactionResult,
    signature: &Signature,
    slot: u64,
    config: &EnhancedLoggingConfig,
    pre_states: Option<&AccountStates>,
    post_states: Option<&AccountStates>,
) -> EnhancedTransactionLog {
    let (status, compute_consumed) = match result {
        Ok(meta) => (TransactionStatus::Success, meta.compute_units_consumed),
        Err(failed) => (
            TransactionStatus::Failed(format!("{:?}", failed.err)),
            failed.meta.compute_units_consumed,
        ),
    };

    let estimated_fee = (transaction.signatures.len() as u64) * 5000;

    // Capture account states if both pre and post states are provided
    let account_states = if let (Some(pre), Some(post)) = (pre_states, post_states) {
        let mut states = HashMap::new();
        for pubkey in &transaction.message.account_keys {
            let (lamports_before, data_len_before) = pre.get(pubkey).copied().unwrap_or((0, 0));
            let (lamports_after, data_len_after) = post.get(pubkey).copied().unwrap_or((0, 0));

            states.insert(
                solana_pubkey::Pubkey::new_from_array(pubkey.to_bytes()),
                AccountStateSnapshot {
                    lamports_before,
                    lamports_after,
                    data_len_before,
                    data_len_after,
                },
            );
        }
        Some(states)
    } else {
        None
    };

    // Build full instructions with accounts and data
    let mut instructions: Vec<EnhancedInstructionLog> = transaction
        .message
        .instructions
        .iter()
        .enumerate()
        .map(|(index, ix)| {
            let program_id = transaction.message.account_keys[ix.program_id_index as usize];
            let mut log = EnhancedInstructionLog::new(
                index,
                solana_pubkey::Pubkey::new_from_array(program_id.to_bytes()),
                get_program_name(&program_id),
            );
            log.accounts = ix
                .accounts
                .iter()
                .map(|&idx| {
                    let pubkey = transaction.message.account_keys[idx as usize];
                    solana_instruction::AccountMeta {
                        pubkey: solana_pubkey::Pubkey::new_from_array(pubkey.to_bytes()),
                        is_signer: transaction.message.is_signer(idx as usize),
                        is_writable: transaction.message.is_maybe_writable(idx as usize, None),
                    }
                })
                .collect();
            log.data = ix.data.clone();
            log
        })
        .collect();

    // Extract inner instructions from LiteSVM metadata
    let inner_instructions_list = match result {
        Ok(meta) => &meta.inner_instructions,
        Err(failed) => &failed.meta.inner_instructions,
    };

    // Apply decoder to instructions if enabled
    if config.decode_light_instructions {
        for instruction in instructions.iter_mut() {
            instruction.decode(config);
        }

        // Populate inner instructions for each top-level instruction
        for (instruction_index, inner_list) in inner_instructions_list.iter().enumerate() {
            if let Some(instruction) = instructions.get_mut(instruction_index) {
                instruction.inner_instructions = parse_inner_instructions(
                    inner_list,
                    &transaction.message.account_keys,
                    &transaction.message,
                    1,
                    config,
                );
            }
        }
    }

    let pretty_logs_string = get_pretty_logs_string(result);

    let sig_bytes: [u8; 64] = signature.as_ref().try_into().unwrap_or([0u8; 64]);
    let mut log = EnhancedTransactionLog::new(
        light_instruction_decoder::solana_signature::Signature::from(sig_bytes),
        slot,
    );
    log.status = status;
    log.fee = estimated_fee;
    log.compute_used = compute_consumed;
    log.instructions = instructions;
    log.program_logs_pretty = pretty_logs_string;
    log.account_states = account_states;
    log
}

/// Parse inner instructions from Solana's InnerInstruction format with proper nesting
fn parse_inner_instructions(
    inner_instructions: &[InnerInstruction],
    account_keys: &[Pubkey],
    message: &solana_sdk::message::Message,
    base_depth: usize,
    config: &EnhancedLoggingConfig,
) -> Vec<EnhancedInstructionLog> {
    let mut result = Vec::new();

    for (index, inner_ix) in inner_instructions.iter().enumerate() {
        let program_id = account_keys[inner_ix.instruction.program_id_index as usize];
        let program_name = get_program_name(&program_id);

        let accounts: Vec<solana_instruction::AccountMeta> = inner_ix
            .instruction
            .accounts
            .iter()
            .map(|&idx| {
                let account_index = idx as usize;
                let pubkey = account_keys[account_index];

                let is_signer = message.is_signer(account_index);
                let is_writable = message.is_maybe_writable(account_index, None);

                solana_instruction::AccountMeta {
                    pubkey: solana_pubkey::Pubkey::new_from_array(pubkey.to_bytes()),
                    is_signer,
                    is_writable,
                }
            })
            .collect();

        let instruction_depth = base_depth + (inner_ix.stack_height as usize).saturating_sub(1);

        let mut instruction_log = EnhancedInstructionLog::new(
            index,
            solana_pubkey::Pubkey::new_from_array(program_id.to_bytes()),
            program_name,
        );
        instruction_log.accounts = accounts;
        instruction_log.data = inner_ix.instruction.data.clone();
        instruction_log.depth = instruction_depth;

        // Decode the instruction if enabled
        if config.decode_light_instructions {
            instruction_log.decode(config);
        }

        // Find the correct parent for this instruction based on stack height
        if inner_ix.stack_height <= 2 {
            result.push(instruction_log);
        } else {
            let target_parent_depth = instruction_depth - 1;
            if let Some(parent) = EnhancedInstructionLog::find_parent_for_instruction(
                &mut result,
                target_parent_depth,
            ) {
                parent.inner_instructions.push(instruction_log);
            } else {
                result.push(instruction_log);
            }
        }
    }

    result
}
