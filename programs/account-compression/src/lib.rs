#![allow(clippy::too_many_arguments)]
pub mod errors;
pub mod instructions;
pub use instructions::*;
pub mod state;
pub use state::*;
pub mod config_accounts;
pub mod utils;
pub use config_accounts::*;
pub mod processor;
pub use processor::*;
pub mod sdk;
use anchor_lang::prelude::*;

declare_id!("5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN");
#[constant]
pub const PROGRAM_ID: &str = "5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN";

#[program]
pub mod account_compression {

    use self::{
        initialize_state_merkle_tree_and_nullifier_queue::process_initialize_state_merkle_tree_and_nullifier_queue,
        insert_into_nullifier_queue::{
            process_insert_into_nullifier_queues, InsertIntoNullifierQueues,
        },
    };

    use super::*;

    pub fn initialize_address_merkle_tree_and_queue<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeAddressMerkleTreeAndQueue<'info>>,
        index: u64,
        owner: Pubkey,
        delegate: Option<Pubkey>,
        address_merkle_tree_config: AddressMerkleTreeConfig,
        address_queue_config: AddressQueueConfig,
    ) -> Result<()> {
        process_initialize_address_merkle_tree_and_queue(
            ctx,
            index,
            owner,
            delegate,
            address_merkle_tree_config,
            address_queue_config,
        )
    }

    pub fn insert_addresses<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InsertAddresses<'info>>,
        addresses: Vec<[u8; 32]>,
    ) -> Result<()> {
        process_insert_addresses(ctx, addresses)
    }
    /// Updates the address Merkle tree with a new address.
    pub fn update_address_merkle_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateMerkleTree<'info>>,
        // Index of the Merkle tree changelog.
        changelog_index: u16,
        // Index of the address to dequeue.
        value: u16,
        // Index of the next address.
        next_index: u64,
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
            value,
            next_index as usize,
            low_address_index,
            low_address_value,
            low_address_next_index,
            low_address_next_value,
            low_address_proof,
        )
    }

    pub fn rollover_address_merkle_tree_and_queue<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, RolloverAddressMerkleTreeAndQueue<'info>>,
    ) -> Result<()> {
        process_rollover_address_merkle_tree_and_queue(ctx)
    }

    /// initialize group (a group can be used to give multiple programs acess to the same Merkle trees by registering the programs to the group)
    pub fn initialize_group_authority<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeGroupAuthority<'info>>,
        _seed: [u8; 32],
        authority: Pubkey,
    ) -> Result<()> {
        set_group_authority(&mut ctx.accounts.group_authority, authority)?;
        ctx.accounts.group_authority.seed = _seed;
        Ok(())
    }

    pub fn update_group_authority<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateGroupAuthority<'info>>,
        authority: Pubkey,
    ) -> Result<()> {
        set_group_authority(&mut ctx.accounts.group_authority, authority)
    }

    pub fn register_program_to_group<'info>(
        ctx: Context<'_, '_, '_, 'info, RegisterProgramToGroup<'info>>,
        program_id: Pubkey,
    ) -> Result<()> {
        process_register_program(ctx, program_id)
    }

    /// Initializes a new Merkle tree from config bytes.
    /// Index is an optional identifier and not checked by the program.
    pub fn initialize_state_merkle_tree_and_nullifier_queue(
        ctx: Context<InitializeStateMerkleTreeAndNullifierQueue>,
        index: u64,
        owner: Pubkey,
        delegate: Option<Pubkey>,
        state_merkle_tree_config: StateMerkleTreeConfig,
        nullifier_queue_config: NullifierQueueConfig,
        // additional rent for the cpi context account
        // so that it can be rolled over as well
        additional_rent: u64,
    ) -> Result<()> {
        process_initialize_state_merkle_tree_and_nullifier_queue(
            ctx,
            index,
            owner,
            delegate,
            state_merkle_tree_config,
            nullifier_queue_config,
            additional_rent,
        )
    }

    pub fn append_leaves_to_merkle_trees<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
        leaves: Vec<(u8, [u8; 32])>,
    ) -> Result<()> {
        process_append_leaves_to_merkle_trees(ctx, leaves)
    }

    pub fn nullify_leaves<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, NullifyLeaves<'info>>,
        change_log_indices: Vec<u64>,
        leaves_queue_indices: Vec<u16>,
        indices: Vec<u64>,
        proofs: Vec<Vec<[u8; 32]>>,
    ) -> Result<()> {
        process_nullify_leaves(
            &ctx,
            &change_log_indices,
            &leaves_queue_indices,
            &indices,
            &proofs,
        )
    }

    pub fn insert_into_nullifier_queues<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InsertIntoNullifierQueues<'info>>,
        elements: Vec<[u8; 32]>,
    ) -> Result<()> {
        process_insert_into_nullifier_queues(ctx, &elements)
    }

    pub fn rollover_state_merkle_tree_and_nullifier_queue<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, RolloverStateMerkleTreeAndNullifierQueue<'info>>,
    ) -> Result<()> {
        process_rollover_state_merkle_tree_nullifier_queue_pair(ctx)
    }

    // TODO: add claim instruction
    // TODO: insert into indexed array just insert into one array instead of possibly multiple

    // TODO: insert_from_nullifier_queue_into_merkle_tree ( to nullify transactions)
}
