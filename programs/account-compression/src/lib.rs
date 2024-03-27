#![allow(clippy::too_many_arguments)]
pub mod errors;
pub mod instructions;
pub use instructions::*;
pub mod state;
pub use state::*;
pub mod config_accounts;
pub mod utils;
pub use config_accounts::*;

declare_id!("5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN");
#[constant]
pub const PROGRAM_ID: &str = "5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN";

#[program]
pub mod account_compression {
    use super::*;

    pub fn initialize_address_queue(_ctx: Context<InitializeAddressQueue>) -> Result<()> {
        Ok(())
    }

    pub fn initialize_address_merkle_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeAddressMerkleTree<'info>>,
        index: u64,
        owner: Pubkey,
        delegate: Option<Pubkey>,
        height: u64,
        changelog_size: u64,
        roots_size: u64,
        canopy_depth: u64,
    ) -> Result<()> {
        process_initialize_address_merkle_tree(
            ctx,
            index,
            owner,
            delegate,
            height,
            changelog_size,
            roots_size,
            canopy_depth,
        )
    }

    pub fn insert_addresses<'info>(
        ctx: Context<'_, '_, '_, 'info, InsertAddresses<'info>>,
        addresses: Vec<[u8; 32]>,
    ) -> Result<()> {
        process_insert_addresses(ctx, addresses)
    }

    // Commented because usize breaks the idl
    // pub fn update_address_merkle_tree<'info>(
    //     ctx: Context<'_, '_, '_, 'info, UpdateMerkleTree<'info>>,
    //     // Index of the Merkle tree changelog.
    //     changelog_index: u16,
    //     // Index of the address to dequeue.
    //     queue_index: u16,
    //     // Index of the next address.
    //     address_next_index: usize,
    //     // Value of the next address.
    //     address_next_value: [u8; 32],
    //     // Low address.
    //     low_address: RawIndexingElement<usize, 32>,
    //     // Value of the next address.
    //     low_address_next_value: [u8; 32],
    //     // Merkle proof for updating the low address.
    //     low_address_proof: [[u8; 32]; 22],
    //     // ZK proof for integrity of provided `address_next_index` and
    //     // `address_next_value`.
    //     next_address_proof: [u8; 128],
    // ) -> Result<()> {
    //     process_update_address_merkle_tree(
    //         ctx,
    //         changelog_index,
    //         queue_index,
    //         address_next_index,
    //         address_next_value,
    //         low_address,
    //         low_address_next_value,
    //         low_address_proof,
    //         next_address_proof,
    //     )
    // }
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
    /// TODO: think the index over
    pub fn initialize_state_merkle_tree(
        ctx: Context<InitializeStateMerkleTree>,
        index: u64,
        owner: Pubkey,
        delegate: Option<Pubkey>,
        height: u64,
        changelog_size: u64,
        roots_size: u64,
        canopy_depth: u64,
        associated_queue: Option<Pubkey>,
    ) -> Result<()> {
        process_initialize_state_merkle_tree(
            ctx,
            index,
            owner,
            delegate,
            height,
            changelog_size,
            roots_size,
            canopy_depth,
            associated_queue,
        )
    }

    pub fn append_leaves_to_merkle_trees<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, AppendLeaves<'info>>,
        leaves: Vec<[u8; 32]>,
    ) -> Result<()> {
        process_append_leaves_to_merkle_trees(ctx, &leaves)
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

    // TODO: add insert into merkle tree function that inserts multiple leaves into a single merkle tree
    pub fn initialize_indexed_array<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InitializeIndexedArrays<'info>>,
        index: u64,
        owner: Pubkey,
        delegate: Option<Pubkey>,
        associated_merkle_tree: Option<Pubkey>,
    ) -> Result<()> {
        process_initialize_indexed_array(ctx, index, owner, delegate, associated_merkle_tree)
    }

    pub fn insert_into_indexed_arrays<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InsertIntoIndexedArrays<'info>>,
        elements: Vec<[u8; 32]>,
    ) -> Result<()> {
        process_insert_into_indexed_arrays(ctx, &elements)
    }

    // TODO: insert into indexed array just insert into one array instead of possibly multiple

    // TODO: insert_from_indexed_array_into_merkle_tree ( to nullify transactions)
}
