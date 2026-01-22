//! Light Registry program instruction decoder.
//!
//! This module provides a macro-derived decoder for the Light Registry program,
//! which uses 8-byte Anchor discriminators.
//!
//! The Registry program manages:
//! - Protocol configuration
//! - Forester registration and epochs
//! - Merkle tree initialization and operations
//! - Rollover operations
//! - Compressible config management

// Allow the macro-generated code to reference types from this crate
extern crate self as light_instruction_decoder;

use light_instruction_decoder_derive::InstructionDecoder;

/// Light Registry program instructions.
///
/// The Registry program uses 8-byte Anchor discriminators computed from
/// sha256("global:<snake_case_instruction_name>").
#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX",
    program_name = "Light Registry",
    discriminator_size = 8
)]
pub enum RegistryInstruction {
    // ========================================================================
    // Protocol Config
    // ========================================================================
    /// Initialize the protocol configuration
    #[instruction_decoder(account_names = ["fee_payer", "authority", "protocol_config_pda", "system_program", "self_program"])]
    InitializeProtocolConfig { bump: u8 },

    /// Update the protocol configuration
    #[instruction_decoder(account_names = ["fee_payer", "authority", "protocol_config_pda", "new_authority"])]
    UpdateProtocolConfig,

    // ========================================================================
    // Forester Management
    // ========================================================================
    /// Register a new forester
    #[instruction_decoder(account_names = ["fee_payer", "authority", "protocol_config_pda", "forester_pda", "system_program"])]
    RegisterForester { bump: u8 },

    /// Update a forester PDA
    #[instruction_decoder(account_names = ["authority", "forester_pda", "new_authority"])]
    UpdateForesterPda,

    /// Update a forester's weight
    #[instruction_decoder(account_names = ["authority", "protocol_config_pda", "forester_pda"])]
    UpdateForesterPdaWeight { new_weight: u64 },

    // ========================================================================
    // Epoch Management
    // ========================================================================
    /// Register a forester for an epoch
    #[instruction_decoder(account_names = ["fee_payer", "authority", "forester_pda", "forester_epoch_pda", "protocol_config", "epoch_pda", "system_program"])]
    RegisterForesterEpoch { epoch: u64 },

    /// Finalize forester registration
    #[instruction_decoder(account_names = ["authority", "forester_epoch_pda", "epoch_pda"])]
    FinalizeRegistration,

    /// Report work done by forester
    #[instruction_decoder(account_names = ["authority", "forester_epoch_pda", "epoch_pda"])]
    ReportWork,

    // ========================================================================
    // System Program Registration
    // ========================================================================
    /// Register a system program
    #[instruction_decoder(account_names = ["authority", "cpi_authority", "program_to_be_registered", "registered_program_pda", "group_pda", "account_compression_program", "system_program"])]
    RegisterSystemProgram { bump: u8 },

    /// Deregister a system program
    #[instruction_decoder(account_names = ["authority", "cpi_authority", "registered_program_pda", "group_pda", "account_compression_program"])]
    DeregisterSystemProgram { bump: u8 },

    // ========================================================================
    // Tree Initialization
    // ========================================================================
    /// Initialize an address Merkle tree
    #[instruction_decoder(account_names = ["authority", "merkle_tree", "queue", "registered_program_pda", "cpi_authority", "account_compression_program", "protocol_config_pda", "cpi_context_account", "light_system_program"])]
    InitializeAddressMerkleTree { bump: u8 },

    /// Initialize a state Merkle tree
    #[instruction_decoder(account_names = ["authority", "merkle_tree", "queue", "registered_program_pda", "cpi_authority", "account_compression_program", "protocol_config_pda", "cpi_context_account", "light_system_program"])]
    InitializeStateMerkleTree { bump: u8 },

    /// Initialize a batched state Merkle tree
    #[instruction_decoder(account_names = ["authority", "merkle_tree", "queue", "registered_program_pda", "cpi_authority", "account_compression_program", "protocol_config_pda", "cpi_context_account", "light_system_program"])]
    InitializeBatchedStateMerkleTree { bump: u8 },

    /// Initialize a batched address Merkle tree
    #[instruction_decoder(account_names = ["authority", "merkle_tree", "registered_program_pda", "cpi_authority", "account_compression_program", "protocol_config_pda"])]
    InitializeBatchedAddressMerkleTree { bump: u8 },

    // ========================================================================
    // Tree Operations
    // ========================================================================
    /// Nullify a leaf in the tree
    #[instruction_decoder(account_names = ["registered_forester_pda", "authority", "cpi_authority", "registered_program_pda", "account_compression_program", "log_wrapper", "merkle_tree", "nullifier_queue"])]
    Nullify { bump: u8 },

    /// Update an address Merkle tree
    #[instruction_decoder(account_names = ["registered_forester_pda", "authority", "cpi_authority", "registered_program_pda", "account_compression_program", "log_wrapper", "merkle_tree", "queue"])]
    UpdateAddressMerkleTree { bump: u8 },

    /// Batch nullify leaves
    #[instruction_decoder(account_names = ["registered_forester_pda", "authority", "cpi_authority", "registered_program_pda", "account_compression_program", "log_wrapper", "merkle_tree"])]
    BatchNullify { bump: u8 },

    /// Batch append to output queue
    #[instruction_decoder(account_names = ["registered_forester_pda", "authority", "cpi_authority", "registered_program_pda", "account_compression_program", "log_wrapper", "merkle_tree", "output_queue"])]
    BatchAppend { bump: u8 },

    /// Batch update an address tree
    #[instruction_decoder(account_names = ["registered_forester_pda", "authority", "cpi_authority", "registered_program_pda", "account_compression_program", "log_wrapper", "merkle_tree"])]
    BatchUpdateAddressTree { bump: u8 },

    // ========================================================================
    // Rollover Operations
    // ========================================================================
    /// Rollover address Merkle tree and queue
    #[instruction_decoder(account_names = ["registered_forester_pda", "authority", "cpi_authority", "registered_program_pda", "account_compression_program", "new_merkle_tree", "new_queue", "old_merkle_tree", "old_queue"])]
    RolloverAddressMerkleTreeAndQueue { bump: u8 },

    /// Rollover state Merkle tree and queue
    #[instruction_decoder(account_names = ["registered_forester_pda", "authority", "cpi_authority", "registered_program_pda", "account_compression_program", "new_merkle_tree", "new_queue", "old_merkle_tree", "old_queue", "cpi_context_account", "light_system_program", "protocol_config_pda"])]
    RolloverStateMerkleTreeAndQueue { bump: u8 },

    /// Rollover batched address Merkle tree
    #[instruction_decoder(account_names = ["registered_forester_pda", "authority", "cpi_authority", "registered_program_pda", "account_compression_program", "new_address_merkle_tree", "old_address_merkle_tree"])]
    RolloverBatchedAddressMerkleTree { bump: u8 },

    /// Rollover batched state Merkle tree
    #[instruction_decoder(account_names = ["registered_forester_pda", "authority", "new_state_merkle_tree", "old_state_merkle_tree", "new_output_queue", "old_output_queue", "cpi_context_account", "registered_program_pda", "cpi_authority", "account_compression_program", "protocol_config_pda", "light_system_program"])]
    RolloverBatchedStateMerkleTree { bump: u8 },

    // ========================================================================
    // Migration
    // ========================================================================
    /// Migrate state
    #[instruction_decoder(account_names = ["registered_forester_pda", "authority", "cpi_authority", "registered_program_pda", "account_compression_program", "merkle_tree"])]
    MigrateState { bump: u8 },

    // ========================================================================
    // Compressible Config
    // ========================================================================
    /// Create a config counter
    #[instruction_decoder(account_names = ["fee_payer", "authority", "protocol_config_pda", "config_counter", "system_program"])]
    CreateConfigCounter,

    /// Create a compressible config
    #[instruction_decoder(account_names = ["fee_payer", "authority", "protocol_config_pda", "config_counter", "compressible_config", "system_program"])]
    CreateCompressibleConfig,

    /// Update a compressible config
    #[instruction_decoder(account_names = ["update_authority", "compressible_config", "new_update_authority", "new_withdrawal_authority"])]
    UpdateCompressibleConfig,

    /// Pause a compressible config (only requires update_authority and compressible_config)
    #[instruction_decoder(account_names = ["update_authority", "compressible_config"])]
    PauseCompressibleConfig,

    /// Unpause a compressible config (only requires update_authority and compressible_config)
    #[instruction_decoder(account_names = ["update_authority", "compressible_config"])]
    UnpauseCompressibleConfig,

    /// Deprecate a compressible config (only requires update_authority and compressible_config)
    #[instruction_decoder(account_names = ["update_authority", "compressible_config"])]
    DeprecateCompressibleConfig,

    // ========================================================================
    // Token Operations
    // ========================================================================
    /// Withdraw from funding pool
    #[instruction_decoder(account_names = ["fee_payer", "withdrawal_authority", "compressible_config", "rent_sponsor", "compression_authority", "destination", "system_program", "compressed_token_program"])]
    WithdrawFundingPool { amount: u64 },

    /// Claim compressed tokens
    #[instruction_decoder(account_names = ["authority", "registered_forester_pda", "rent_sponsor", "compression_authority", "compressible_config", "compressed_token_program"])]
    Claim,

    /// Compress and close token account
    #[instruction_decoder(account_names = ["authority", "registered_forester_pda", "compression_authority", "compressible_config"])]
    CompressAndClose {
        authority_index: u8,
        destination_index: u8,
    },
}
