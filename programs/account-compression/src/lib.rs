#![allow(clippy::too_many_arguments)]
#![allow(unexpected_cfgs)]
#![allow(deprecated)]
pub mod errors;
pub mod instructions;
pub use instructions::*;
pub mod state;
pub use state::*;
pub mod context;
pub mod processor;
pub mod utils;
use anchor_lang::prelude::*;
use errors::AccountCompressionErrorCode;
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::{InstructionDataBatchAppendInputs, InstructionDataBatchNullifyInputs},
};

declare_id!("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "account-compression",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol"
}

#[program]
pub mod account_compression {

    use super::*;
    use crate::processor::insert_into_queues::process_insert_into_queues;

    pub fn initialize_address_merkle_tree_and_queue<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeAddressMerkleTreeAndQueue<'info>>,
        index: u64,
        program_owner: Option<Pubkey>,
        forester: Option<Pubkey>,
        address_merkle_tree_config: AddressMerkleTreeConfig,
        address_queue_config: AddressQueueConfig,
    ) -> Result<()> {
        process_initialize_address_merkle_tree_and_queue(
            ctx,
            index,
            program_owner,
            forester,
            address_merkle_tree_config,
            address_queue_config,
        )
    }

    /// Updates the address Merkle tree with a new address.
    pub fn update_address_merkle_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateAddressMerkleTree<'info>>,
        // Index of the Merkle tree changelog.
        changelog_index: u16,
        indexed_changelog_index: u16,
        // Index of the address to dequeue.
        value: u16,
        // Low address.
        low_address_index: u64,
        low_address_value: [u8; 32],
        low_address_next_index: u64,
        // Value of the next address.
        low_address_next_value: [u8; 32],
        // Merkle proof for updating the low address.
        low_address_proof: [[u8; 32]; 16],
    ) -> Result<()> {
        process_update_address_merkle_tree(
            ctx,
            changelog_index,
            indexed_changelog_index,
            value,
            low_address_value,
            low_address_next_index,
            low_address_next_value,
            low_address_index,
            low_address_proof,
        )
    }

    pub fn rollover_address_merkle_tree_and_queue<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, RolloverAddressMerkleTreeAndQueue<'info>>,
    ) -> Result<()> {
        process_rollover_address_merkle_tree_and_queue(ctx)
    }

    /// initialize group (a group can be used to give multiple programs access
    /// to the same Merkle trees by registering the programs to the group)
    pub fn initialize_group_authority<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeGroupAuthority<'info>>,
        authority: Pubkey,
    ) -> Result<()> {
        let seed_pubkey = ctx.accounts.seed.key();
        set_group_authority(
            &mut ctx.accounts.group_authority,
            authority,
            Some(seed_pubkey),
        )?;
        Ok(())
    }

    pub fn update_group_authority<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateGroupAuthority<'info>>,
        authority: Pubkey,
    ) -> Result<()> {
        set_group_authority(&mut ctx.accounts.group_authority, authority, None)
    }

    pub fn register_program_to_group<'info>(
        ctx: Context<'_, '_, '_, 'info, RegisterProgramToGroup<'info>>,
    ) -> Result<()> {
        process_register_program(ctx)
    }

    pub fn deregister_program(_ctx: Context<DeregisterProgram>) -> Result<()> {
        Ok(())
    }

    pub fn resize_registered_program_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, ResizeRegisteredProgramPda<'info>>,
    ) -> Result<()> {
        process_resize_registered_program_pda(ctx)
    }

    /// Initializes a new Merkle tree from config bytes.
    /// Index is an optional identifier and not checked by the program.
    pub fn initialize_state_merkle_tree_and_nullifier_queue<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeStateMerkleTreeAndNullifierQueue<'info>>,
        index: u64,
        program_owner: Option<Pubkey>,
        forester: Option<Pubkey>,
        state_merkle_tree_config: StateMerkleTreeConfig,
        nullifier_queue_config: NullifierQueueConfig,
        // additional rent for the cpi context account
        // so that it can be rolled over as well
        additional_bytes: u64,
    ) -> Result<()> {
        if additional_bytes != 0 {
            msg!("additional_bytes is not supported yet");
            return err!(AccountCompressionErrorCode::UnsupportedAdditionalBytes);
        }
        process_initialize_state_merkle_tree_and_nullifier_queue(
            ctx,
            index,
            program_owner,
            forester,
            state_merkle_tree_config,
            nullifier_queue_config,
            additional_bytes,
        )
    }

    /// Inserts nullifiers, leaves, and addresses
    /// into v1 and batched Merkle trees.
    pub fn insert_into_queues<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, GenericInstruction<'info>>,
        bytes: Vec<u8>,
    ) -> Result<()> {
        process_insert_into_queues(&ctx, bytes)
    }

    pub fn nullify_leaves<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, NullifyLeaves<'info>>,
        change_log_indices: Vec<u64>,
        leaves_queue_indices: Vec<u16>,
        leaf_indices: Vec<u64>,
        proofs: Vec<Vec<[u8; 32]>>,
    ) -> Result<()> {
        process_nullify_leaves(
            &ctx,
            &change_log_indices,
            &leaves_queue_indices,
            &leaf_indices,
            &proofs,
        )
    }

    pub fn rollover_state_merkle_tree_and_nullifier_queue<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, RolloverStateMerkleTreeAndNullifierQueue<'info>>,
    ) -> Result<()> {
        process_rollover_state_merkle_tree_nullifier_queue_pair(ctx)
    }

    /// Initialize a batched state Merkle tree account and
    /// an output queue account.
    /// 1. append output state
    ///     State is first inserted into the output queue with
    ///     the instruction append_leaves_to_merkle_trees.
    /// 2. batch append
    ///     Leaves are inserted from the output queue into the
    ///     state Merkle tree with the instruction batch_append.
    /// 3. nullify (input) state
    ///     State is nullified by inserting nullifiers into the
    ///     input queue (part of the Merkle tree account).
    ///     Nullifiers are inserted with the instruction
    ///     insert_into_nullifier_queues.
    /// 4. batch nullify
    ///    Nullifiers are inserted from the input queue into the
    ///    state Merkle tree with the instruction batch_nullify.
    pub fn initialize_batched_state_merkle_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeBatchedStateMerkleTreeAndQueue<'info>>,
        bytes: Vec<u8>,
    ) -> Result<()> {
        let params = InitStateTreeAccountsInstructionData::try_from_slice(&bytes)
            .map_err(|_| AccountCompressionErrorCode::InputDeserializationFailed)?;
        process_initialize_batched_state_merkle_tree(ctx, params)
    }

    /// Initialize a batched address Merkle tree account.
    /// A batched address Merkle tree is an indexed Merkle tree.
    /// Addresses are first inserted into a queue and inserted into
    /// the indexed Merkle tree in batches with a zkp.
    /// The queue is part of the address Merkle tree account.
    /// 1. queue addresses
    ///     Addresses are inserted into the queue with the instruction
    ///     insert_addresses.
    /// 2. update address tree
    ///     The address tree is updated with the instruction
    ///     batch_update_address_tree.
    pub fn initialize_batched_address_merkle_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeBatchedAddressMerkleTree<'info>>,
        bytes: Vec<u8>,
    ) -> Result<()> {
        let params = InitAddressTreeAccountsInstructionData::try_from_slice(&bytes)
            .map_err(|_| AccountCompressionErrorCode::InputDeserializationFailed)?;
        process_initialize_batched_address_merkle_tree(ctx, params)
    }

    /// Nullify a batch of leaves from the input queue
    /// to a batched Merkle tree with a zkp.
    pub fn batch_nullify<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, BatchNullify<'info>>,
        data: Vec<u8>,
    ) -> Result<()> {
        let instruction_data = InstructionDataBatchNullifyInputs::try_from_slice(&data)
            .map_err(|_| AccountCompressionErrorCode::InputDeserializationFailed)?;
        process_batch_nullify(&ctx, instruction_data)
    }

    /// Append a batch of leaves from an output queue
    /// to a batched Merkle tree with a zkp.
    pub fn batch_append<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, BatchAppend<'info>>,
        data: Vec<u8>,
    ) -> Result<()> {
        let instruction_data = InstructionDataBatchAppendInputs::try_from_slice(&data)
            .map_err(|_| AccountCompressionErrorCode::InputDeserializationFailed)?;
        process_batch_append_leaves(&ctx, instruction_data)
    }

    /// Insert a batch of addresses into a
    /// batched address Merkle tree with a zkp.
    pub fn batch_update_address_tree<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, BatchUpdateAddressTree<'info>>,
        data: Vec<u8>,
    ) -> Result<()> {
        let instruction_data = InstructionDataBatchNullifyInputs::try_from_slice(&data)
            .map_err(|_| AccountCompressionErrorCode::InputDeserializationFailed)?;
        process_batch_update_address_tree(&ctx, instruction_data)
    }

    /// Rollover batched address Merkle tree.
    /// Rollover means creating a new Merkle tree accounts
    /// with the parameters of the old account.
    /// Rent is reimbursed from the old account to the payer.
    pub fn rollover_batched_address_merkle_tree<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, RolloverBatchedAddressMerkleTree<'info>>,
        network_fee: Option<u64>,
    ) -> Result<()> {
        process_rollover_batched_address_merkle_tree(ctx, network_fee)
    }

    /// Rollover batched state Merkle tree.
    /// Rollover means creating new queue and Merkle tree accounts
    /// with the parameters of the old accounts.
    /// Rent is reimbursed from the old output queue account to the payer.
    pub fn rollover_batched_state_merkle_tree<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, RolloverBatchedStateMerkleTree<'info>>,
        additional_bytes: u64,
        network_fee: Option<u64>,
    ) -> Result<()> {
        process_rollover_batched_state_merkle_tree(ctx, additional_bytes, network_fee)
    }

    /// Migrate state from a v1 state Merkle tree
    /// to a v2 state Merkle tree.
    pub fn migrate_state<'a, 'b, 'c: 'info, 'info>(
        _ctx: Context<'a, 'b, 'c, 'info, MigrateState<'info>>,
        _input: MigrateLeafParams,
    ) -> Result<()> {
        #[cfg(feature = "migrate-state")]
        {
            process_migrate_state(&_ctx, _input)
        }
        #[cfg(not(feature = "migrate-state"))]
        {
            unimplemented!("migrate_state is disabled")
        }
    }
}
