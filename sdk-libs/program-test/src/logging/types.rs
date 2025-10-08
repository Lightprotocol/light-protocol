//! Type definitions for enhanced logging

use solana_sdk::{
    inner_instruction::InnerInstruction, instruction::AccountMeta, pubkey::Pubkey,
    signature::Signature, system_program,
};

use super::config::EnhancedLoggingConfig;

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
    pub parsed_data: Option<ParsedInstructionData>,
    pub inner_instructions: Vec<EnhancedInstructionLog>,
    pub compute_consumed: Option<u64>,
    pub success: bool,
    pub depth: usize,
}

/// Parsed instruction data for known programs
#[derive(Debug, Clone)]
pub enum ParsedInstructionData {
    LightSystemProgram {
        instruction_type: String,
        compressed_accounts: Option<CompressedAccountSummary>,
        proof_info: Option<ProofSummary>,
        address_params: Option<Vec<AddressParam>>,
        fee_info: Option<FeeSummary>,
        input_account_data: Option<Vec<InputAccountData>>,
        output_account_data: Option<Vec<OutputAccountData>>,
    },
    ComputeBudget {
        instruction_type: String,
        value: Option<u64>,
    },
    System {
        instruction_type: String,
        lamports: Option<u64>,
        space: Option<u64>,
        new_account: Option<Pubkey>,
    },
    Unknown {
        program_name: String,
        data_preview: String,
    },
}

/// Summary of compressed accounts in a Light Protocol instruction
#[derive(Debug, Clone)]
pub struct CompressedAccountSummary {
    pub input_accounts: usize,
    pub output_accounts: usize,
    pub lamports_change: Option<i64>,
}

/// Summary of proof information
#[derive(Debug, Clone)]
pub struct ProofSummary {
    pub proof_type: String,
    pub has_validity_proof: bool,
}

/// Summary of fee information
#[derive(Debug, Clone)]
pub struct FeeSummary {
    pub relay_fee: Option<u64>,
    pub compression_fee: Option<u64>,
}

/// Address assignment state
#[derive(Debug, Clone)]
pub enum AddressAssignment {
    /// V1 address param (no assignment tracking)
    V1,
    /// Not assigned to any output account
    None,
    /// Assigned to output account at index
    AssignedIndex(u8),
}

/// Address parameter information
#[derive(Debug, Clone)]
pub struct AddressParam {
    pub seed: [u8; 32],
    pub address_queue_index: Option<u8>,
    pub address_queue_pubkey: Option<solana_sdk::pubkey::Pubkey>,
    pub merkle_tree_index: Option<u8>,
    pub address_merkle_tree_pubkey: Option<solana_sdk::pubkey::Pubkey>,
    pub root_index: Option<u16>,
    pub derived_address: Option<[u8; 32]>,
    pub assigned_account_index: AddressAssignment,
}

/// Input account data
#[derive(Debug, Clone)]
pub struct InputAccountData {
    pub lamports: u64,
    pub owner: Option<solana_sdk::pubkey::Pubkey>,
    pub merkle_tree_index: Option<u8>,
    pub merkle_tree_pubkey: Option<solana_sdk::pubkey::Pubkey>,
    pub queue_index: Option<u8>,
    pub queue_pubkey: Option<solana_sdk::pubkey::Pubkey>,
    pub address: Option<[u8; 32]>,
    pub data_hash: Vec<u8>,
    pub discriminator: Vec<u8>,
    pub leaf_index: Option<u32>,
    pub root_index: Option<u16>,
}

/// Output account data
#[derive(Debug, Clone)]
pub struct OutputAccountData {
    pub lamports: u64,
    pub data: Option<Vec<u8>>,
    pub owner: Option<solana_sdk::pubkey::Pubkey>,
    pub merkle_tree_index: Option<u8>,
    pub merkle_tree_pubkey: Option<solana_sdk::pubkey::Pubkey>,
    pub queue_index: Option<u8>,
    pub queue_pubkey: Option<solana_sdk::pubkey::Pubkey>,
    pub address: Option<[u8; 32]>,
    pub data_hash: Vec<u8>,
    pub discriminator: Vec<u8>,
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

impl EnhancedTransactionLog {
    /// Use LiteSVM's pretty logs instead of parsing raw logs
    fn get_pretty_logs_string(result: &litesvm::types::TransactionResult) -> String {
        match result {
            Ok(meta) => meta.pretty_logs(),
            Err(failed) => failed.meta.pretty_logs(),
        }
    }

    /// Create from LiteSVM transaction result
    pub fn from_transaction_result(
        transaction: &solana_sdk::transaction::Transaction,
        result: &litesvm::types::TransactionResult,
        signature: &Signature,
        slot: u64,
        config: &EnhancedLoggingConfig,
    ) -> Self {
        let (status, compute_consumed) = match result {
            Ok(meta) => (TransactionStatus::Success, meta.compute_units_consumed),
            Err(failed) => (
                TransactionStatus::Failed(format!("{:?}", failed.err)),
                failed.meta.compute_units_consumed,
            ),
        };

        // Calculate estimated fee (basic calculation: signatures * lamports_per_signature)
        // Default Solana fee is 5000 lamports per signature
        let estimated_fee = (transaction.signatures.len() as u64) * 5000;

        // Parse instructions
        let instructions: Vec<EnhancedInstructionLog> = transaction
            .message
            .instructions
            .iter()
            .enumerate()
            .map(|(index, ix)| EnhancedInstructionLog {
                index,
                program_id: transaction.message.account_keys[ix.program_id_index as usize],
                program_name: get_program_name(
                    &transaction.message.account_keys[ix.program_id_index as usize],
                ),
                instruction_name: None, // Will be filled by decoder
                accounts: ix
                    .accounts
                    .iter()
                    .map(|&idx| AccountMeta {
                        pubkey: transaction.message.account_keys[idx as usize],
                        is_signer: transaction.message.is_signer(idx as usize),
                        is_writable: transaction.message.is_maybe_writable(idx as usize, None),
                    })
                    .collect(),
                data: ix.data.clone(),
                parsed_data: None,              // Will be filled by decoder
                inner_instructions: Vec::new(), // Will be filled from meta
                compute_consumed: None,
                success: true,
                depth: 0,
            })
            .collect();

        // Extract inner instructions from LiteSVM metadata
        let inner_instructions_list = match result {
            Ok(meta) => &meta.inner_instructions,
            Err(failed) => &failed.meta.inner_instructions,
        };

        // Apply decoder to instructions if enabled and populate inner instructions
        let mut instructions = instructions;
        if config.decode_light_instructions {
            // First, decode all top-level instructions
            for instruction in instructions.iter_mut() {
                instruction.parsed_data = super::decoder::decode_instruction(
                    &instruction.program_id,
                    &instruction.data,
                    &instruction.accounts,
                );
                if let Some(ref parsed) = instruction.parsed_data {
                    instruction.instruction_name = match parsed {
                        ParsedInstructionData::LightSystemProgram {
                            instruction_type, ..
                        } => Some(instruction_type.clone()),
                        ParsedInstructionData::ComputeBudget {
                            instruction_type, ..
                        } => Some(instruction_type.clone()),
                        ParsedInstructionData::System {
                            instruction_type, ..
                        } => Some(instruction_type.clone()),
                        _ => None,
                    };
                }
            }

            // Now populate inner instructions for each top-level instruction
            for (instruction_index, inner_list) in inner_instructions_list.iter().enumerate() {
                if let Some(instruction) = instructions.get_mut(instruction_index) {
                    instruction.inner_instructions = Self::parse_inner_instructions(
                        inner_list, // inner_list is already Vec<InnerInstruction>
                        &transaction.message.account_keys,
                        &transaction.message, // Pass the full message for account access info
                        1,                    // Start at depth 1 for inner instructions
                        config,
                    );
                }
            }
        }

        // Get LiteSVM's pretty formatted logs
        let pretty_logs_string = Self::get_pretty_logs_string(result);

        Self {
            signature: *signature,
            slot,
            status,
            fee: estimated_fee,
            compute_used: compute_consumed,
            compute_total: 1_400_000, // Default compute limit
            instructions,
            account_changes: Vec::new(), // Will be filled if requested
            program_logs_pretty: pretty_logs_string,
            light_events: Vec::new(),
        }
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

            let accounts: Vec<AccountMeta> = inner_ix
                .instruction
                .accounts
                .iter()
                .map(|&idx| {
                    let account_index = idx as usize;
                    let pubkey = account_keys[account_index];

                    // Get the correct signer and writable information from the original transaction message
                    let is_signer = message.is_signer(account_index);
                    let is_writable = message.is_maybe_writable(account_index, None);

                    AccountMeta {
                        pubkey,
                        is_signer,
                        is_writable,
                    }
                })
                .collect();

            let parsed_data = if config.decode_light_instructions {
                super::decoder::decode_instruction(
                    &program_id,
                    &inner_ix.instruction.data,
                    &accounts,
                )
            } else {
                None
            };

            let instruction_name = parsed_data.as_ref().and_then(|parsed| match parsed {
                ParsedInstructionData::LightSystemProgram {
                    instruction_type, ..
                } => Some(instruction_type.clone()),
                ParsedInstructionData::ComputeBudget {
                    instruction_type, ..
                } => Some(instruction_type.clone()),
                ParsedInstructionData::System {
                    instruction_type, ..
                } => Some(instruction_type.clone()),
                _ => None,
            });

            // Calculate the actual depth based on stack_height
            // stack_height 2 = first level CPI (depth = base_depth + 1)
            // stack_height 3 = second level CPI (depth = base_depth + 2), etc.
            let instruction_depth = base_depth + (inner_ix.stack_height as usize).saturating_sub(1);

            let instruction_log = EnhancedInstructionLog {
                index,
                program_id,
                program_name,
                instruction_name,
                accounts,
                data: inner_ix.instruction.data.clone(),
                parsed_data,
                inner_instructions: Vec::new(),
                compute_consumed: None,
                success: true, // We assume inner instructions succeeded if we're parsing them
                depth: instruction_depth,
            };

            // Find the correct parent for this instruction based on stack height
            // Stack height 2 = direct CPI, should be at top level
            // Stack height 3+ = nested CPI, should be child of previous instruction with stack_height - 1
            if inner_ix.stack_height <= 2 {
                // Top-level CPI - add directly to result
                result.push(instruction_log);
            } else {
                // Nested CPI - find the appropriate parent
                // We need to traverse the result structure to find the right parent
                let target_parent_depth = instruction_depth - 1;
                if let Some(parent) =
                    Self::find_parent_for_instruction(&mut result, target_parent_depth)
                {
                    parent.inner_instructions.push(instruction_log);
                } else {
                    // Fallback: add to top level if we can't find appropriate parent
                    result.push(instruction_log);
                }
            }
        }

        result
    }

    /// Helper function to find the appropriate parent for nested instructions
    fn find_parent_for_instruction(
        instructions: &mut [EnhancedInstructionLog],
        target_depth: usize,
    ) -> Option<&mut EnhancedInstructionLog> {
        for instruction in instructions.iter_mut().rev() {
            if instruction.depth == target_depth {
                return Some(instruction);
            }
            // Recursively search in inner instructions
            if let Some(parent) =
                Self::find_parent_for_instruction(&mut instruction.inner_instructions, target_depth)
            {
                return Some(parent);
            }
        }
        None
    }
}
/// Get human-readable program name from pubkey
fn get_program_name(program_id: &Pubkey) -> String {
    match program_id.to_string().as_str() {
        id if id == system_program::ID.to_string() => "System Program".to_string(),
        "ComputeBudget111111111111111111111111111111" => "Compute Budget".to_string(),
        "SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7" => "Light System Program".to_string(),
        "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq" => "Account Compression".to_string(),
        "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m" => "Compressed Token Program".to_string(),
        _ => format!("Unknown Program ({})", program_id),
    }
}
