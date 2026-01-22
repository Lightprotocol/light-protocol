//! Type definitions for enhanced logging
//!
//! This module contains all the data types used for instruction decoding
//! and transaction logging. These types are independent of any test framework
//! (LiteSVM, etc.) and can be used in standalone tools.

use std::collections::HashMap;

use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use solana_signature::Signature;

use crate::{DecodedInstruction, DecoderRegistry, EnhancedLoggingConfig};

/// Pre and post transaction account state snapshot
#[derive(Debug, Clone, Default)]
pub struct AccountStateSnapshot {
    pub lamports_before: u64,
    pub lamports_after: u64,
    pub data_len_before: usize,
    pub data_len_after: usize,
}

/// Enhanced transaction log containing all formatting information
#[derive(Debug, Clone)]
pub struct EnhancedTransactionLog {
    pub signature: Signature,
    pub slot: u64,
    pub status: TransactionStatus,
    pub fee: u64,
    pub compute_used: u64,
    pub compute_total: u64,
    pub instructions: Vec<EnhancedInstructionLog>,
    pub account_changes: Vec<AccountChange>,
    pub program_logs_pretty: String,
    pub light_events: Vec<LightProtocolEvent>,
    /// Pre and post transaction account state snapshots (keyed by pubkey)
    pub account_states: Option<HashMap<Pubkey, AccountStateSnapshot>>,
}

impl EnhancedTransactionLog {
    /// Create a new empty transaction log with basic info
    pub fn new(signature: Signature, slot: u64) -> Self {
        Self {
            signature,
            slot,
            status: TransactionStatus::Unknown,
            fee: 0,
            compute_used: 0,
            compute_total: 1_400_000,
            instructions: Vec::new(),
            account_changes: Vec::new(),
            program_logs_pretty: String::new(),
            light_events: Vec::new(),
            account_states: None,
        }
    }
}

/// Transaction execution status
#[derive(Debug, Clone)]
pub enum TransactionStatus {
    Success,
    Failed(String),
    Unknown,
}

impl TransactionStatus {
    pub fn text(&self) -> String {
        match self {
            TransactionStatus::Success => "Success".to_string(),
            TransactionStatus::Failed(err) => format!("Failed: {}", err),
            TransactionStatus::Unknown => "Unknown".to_string(),
        }
    }
}

/// Enhanced instruction log with hierarchy and parsing
#[derive(Debug, Clone)]
pub struct EnhancedInstructionLog {
    pub index: usize,
    pub program_id: Pubkey,
    pub program_name: String,
    pub instruction_name: Option<String>,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
    /// Decoded instruction from custom decoder (if available)
    pub decoded_instruction: Option<DecodedInstruction>,
    pub inner_instructions: Vec<EnhancedInstructionLog>,
    pub compute_consumed: Option<u64>,
    pub success: bool,
    pub depth: usize,
}

impl EnhancedInstructionLog {
    /// Create a new instruction log
    pub fn new(index: usize, program_id: Pubkey, program_name: String) -> Self {
        Self {
            index,
            program_id,
            program_name,
            instruction_name: None,
            accounts: Vec::new(),
            data: Vec::new(),
            decoded_instruction: None,
            inner_instructions: Vec::new(),
            compute_consumed: None,
            success: true,
            depth: 0,
        }
    }

    /// Decode this instruction using the provided config's decoder registry
    pub fn decode(&mut self, config: &EnhancedLoggingConfig) {
        if !config.decode_light_instructions {
            return;
        }

        // Try the decoder registry (includes custom decoders)
        if let Some(registry) = config.decoder_registry() {
            if let Some((decoded, decoder)) =
                registry.decode(&self.program_id, &self.data, &self.accounts)
            {
                self.instruction_name = Some(decoded.name.clone());
                self.decoded_instruction = Some(decoded);
                self.program_name = decoder.program_name().to_string();
            }
        }
    }

    /// Find parent instruction at target depth for nesting
    pub fn find_parent_for_instruction(
        instructions: &mut [EnhancedInstructionLog],
        target_depth: usize,
    ) -> Option<&mut EnhancedInstructionLog> {
        for instruction in instructions.iter_mut().rev() {
            if instruction.depth == target_depth {
                return Some(instruction);
            }
            if let Some(parent) =
                Self::find_parent_for_instruction(&mut instruction.inner_instructions, target_depth)
            {
                return Some(parent);
            }
        }
        None
    }
}

/// Account state changes during transaction
#[derive(Debug, Clone)]
pub struct AccountChange {
    pub pubkey: Pubkey,
    pub account_type: String,
    pub access: AccountAccess,
    pub account_index: usize,
    pub lamports_before: u64,
    pub lamports_after: u64,
    pub data_len_before: usize,
    pub data_len_after: usize,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: u64,
}

/// Account access pattern during transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountAccess {
    Readonly,
    Writable,
    Signer,
    SignerWritable,
}

impl AccountAccess {
    pub fn symbol(&self, index: usize) -> String {
        format!("#{}", index)
    }

    pub fn text(&self) -> &'static str {
        match self {
            AccountAccess::Readonly => "readonly",
            AccountAccess::Writable => "writable",
            AccountAccess::Signer => "signer",
            AccountAccess::SignerWritable => "signer+writable",
        }
    }
}

/// Light Protocol specific events
#[derive(Debug, Clone)]
pub struct LightProtocolEvent {
    pub event_type: String,
    pub compressed_accounts: Vec<CompressedAccountInfo>,
    pub merkle_tree_changes: Vec<MerkleTreeChange>,
    pub nullifiers: Vec<String>,
}

/// Compressed account information
#[derive(Debug, Clone)]
pub struct CompressedAccountInfo {
    pub hash: String,
    pub owner: Pubkey,
    pub lamports: u64,
    pub data: Option<Vec<u8>>,
    pub address: Option<String>,
}

/// Merkle tree state change
#[derive(Debug, Clone)]
pub struct MerkleTreeChange {
    pub tree_pubkey: Pubkey,
    pub tree_type: String,
    pub sequence_number: u64,
    pub leaf_index: u64,
}

/// Get human-readable program name from pubkey
///
/// First consults the decoder registry if provided, then falls back to hardcoded mappings.
pub fn get_program_name(program_id: &Pubkey, registry: Option<&DecoderRegistry>) -> String {
    // First try to get the name from the decoder registry
    if let Some(reg) = registry {
        if let Some(decoder) = reg.get_decoder(program_id) {
            return decoder.program_name().to_string();
        }
    }

    // Fall back to hardcoded mappings for programs without decoders
    match program_id.to_string().as_str() {
        "11111111111111111111111111111111" => "System Program".to_string(),
        "ComputeBudget111111111111111111111111111111" => "Compute Budget".to_string(),
        "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7" => "Light System Program".to_string(),
        "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq" => "Account Compression".to_string(),
        "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m" => "Light Token Program".to_string(),
        _ => format!("Unknown Program ({})", program_id),
    }
}
