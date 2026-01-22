//! Account Compression program instruction decoder.
//!
//! This module provides a macro-derived decoder for the Account Compression program,
//! which uses 8-byte Anchor discriminators.
//!
//! The Account Compression program manages:
//! - Group authority and program registration
//! - State Merkle tree initialization and operations
//! - Address Merkle tree initialization and operations
//! - Batched tree operations with ZK proofs
//! - Tree rollover operations
//! - State migration

// Allow the macro-generated code to reference types from this crate
extern crate self as light_instruction_decoder;

use light_instruction_decoder_derive::InstructionDecoder;

/// Account Compression program instructions.
///
/// The Account Compression program uses 8-byte Anchor discriminators computed from
/// sha256("global:<snake_case_instruction_name>").
#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq",
    program_name = "Account Compression",
    discriminator_size = 8
)]
pub enum AccountCompressionInstruction {
    // ========================================================================
    // Group Authority Management
    // ========================================================================
    /// Initialize a group authority (allows multiple programs to share Merkle trees)
    #[instruction_decoder(account_names = ["authority", "seed", "group_authority", "system_program"])]
    InitializeGroupAuthority,

    /// Update the group authority
    #[instruction_decoder(account_names = ["authority", "group_authority"])]
    UpdateGroupAuthority,

    // ========================================================================
    // Program Registration
    // ========================================================================
    /// Register a program to a group
    #[instruction_decoder(account_names = ["authority", "program_to_be_registered", "registered_program_pda", "group_authority_pda", "system_program"])]
    RegisterProgramToGroup,

    /// Deregister a program from its group
    #[instruction_decoder(account_names = ["authority", "registered_program_pda", "group_authority_pda", "close_recipient"])]
    DeregisterProgram,

    /// Resize a registered program PDA (v1 to v2 migration)
    #[instruction_decoder(account_names = ["authority", "registered_program_pda", "system_program"])]
    ResizeRegisteredProgramPda,

    // ========================================================================
    // State Tree Operations (v1 - concurrent Merkle tree)
    // ========================================================================
    /// Initialize a state Merkle tree and nullifier queue
    #[instruction_decoder(account_names = ["authority", "merkle_tree", "nullifier_queue", "registered_program_pda"])]
    InitializeStateMerkleTreeAndNullifierQueue,

    /// Nullify leaves in a state Merkle tree
    #[instruction_decoder(account_names = ["authority", "registered_program_pda", "log_wrapper", "merkle_tree", "nullifier_queue"])]
    NullifyLeaves,

    /// Rollover a state Merkle tree and nullifier queue
    #[instruction_decoder(account_names = ["fee_payer", "authority", "registered_program_pda", "new_state_merkle_tree", "new_nullifier_queue", "old_state_merkle_tree", "old_nullifier_queue"])]
    RolloverStateMerkleTreeAndNullifierQueue,

    // ========================================================================
    // Address Tree Operations (v1 - indexed Merkle tree)
    // ========================================================================
    /// Initialize an address Merkle tree and queue
    #[instruction_decoder(account_names = ["authority", "merkle_tree", "queue", "registered_program_pda"])]
    InitializeAddressMerkleTreeAndQueue,

    /// Update an address Merkle tree with a new address
    #[instruction_decoder(account_names = ["authority", "registered_program_pda", "queue", "merkle_tree", "log_wrapper"])]
    UpdateAddressMerkleTree,

    /// Rollover an address Merkle tree and queue
    #[instruction_decoder(account_names = ["fee_payer", "authority", "registered_program_pda", "new_address_merkle_tree", "new_queue", "old_address_merkle_tree", "old_queue"])]
    RolloverAddressMerkleTreeAndQueue,

    // ========================================================================
    // Queue Operations
    // ========================================================================
    /// Insert nullifiers, leaves, and addresses into v1 and batched Merkle trees
    #[instruction_decoder(account_names = ["authority"])]
    InsertIntoQueues,

    // ========================================================================
    // Batched Tree Operations (v2 - with ZK proofs)
    // ========================================================================
    /// Initialize a batched state Merkle tree and output queue
    #[instruction_decoder(account_names = ["authority", "merkle_tree", "queue", "registered_program_pda"])]
    InitializeBatchedStateMerkleTree,

    /// Initialize a batched address Merkle tree
    #[instruction_decoder(account_names = ["authority", "merkle_tree", "registered_program_pda"])]
    InitializeBatchedAddressMerkleTree,

    /// Nullify a batch of leaves from input queue to state Merkle tree with ZK proof
    #[instruction_decoder(account_names = ["authority", "registered_program_pda", "log_wrapper", "merkle_tree"])]
    BatchNullify,

    /// Append a batch of leaves from output queue to state Merkle tree with ZK proof
    #[instruction_decoder(account_names = ["authority", "registered_program_pda", "log_wrapper", "merkle_tree", "output_queue"])]
    BatchAppend,

    /// Insert a batch of addresses into a batched address Merkle tree with ZK proof
    #[instruction_decoder(account_names = ["authority", "registered_program_pda", "log_wrapper", "merkle_tree"])]
    BatchUpdateAddressTree,

    // ========================================================================
    // Batched Rollover Operations
    // ========================================================================
    /// Rollover a batched address Merkle tree
    #[instruction_decoder(account_names = ["fee_payer", "authority", "registered_program_pda", "new_address_merkle_tree", "old_address_merkle_tree"])]
    RolloverBatchedAddressMerkleTree,

    /// Rollover a batched state Merkle tree and output queue
    #[instruction_decoder(account_names = ["fee_payer", "authority", "registered_program_pda", "new_state_merkle_tree", "old_state_merkle_tree", "new_output_queue", "old_output_queue"])]
    RolloverBatchedStateMerkleTree,

    // ========================================================================
    // Migration
    // ========================================================================
    /// Migrate state from a v1 state Merkle tree to a v2 state Merkle tree
    #[instruction_decoder(account_names = ["authority", "registered_program_pda", "log_wrapper", "merkle_tree", "output_queue"])]
    MigrateState,
}
