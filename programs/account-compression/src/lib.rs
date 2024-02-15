use light_indexed_merkle_tree::array::RawIndexingElement;

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
    ) -> Result<()> {
        process_initialize_address_merkle_tree(ctx)
    }

    pub fn insert_addresses<'info>(
        ctx: Context<'_, '_, '_, 'info, InsertAddresses<'info>>,
        addresses: Vec<[u8; 32]>,
    ) -> Result<()> {
        process_insert_addresses(ctx, addresses)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_address_merkle_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, UpdateMerkleTree<'info>>,
        // Index of the Merkle tree changelog.
        changelog_index: u16,
        // Index of the address to dequeue.
        queue_index: u16,
        // Index of the next address.
        address_next_index: usize,
        // Value of the next address.
        address_next_value: [u8; 32],
        // Low address.
        low_address: RawIndexingElement<usize, 32>,
        // Value of the next address.
        low_address_next_value: [u8; 32],
        // Merkle proof for updating the low address.
        low_address_proof: [[u8; 32]; 22],
        // ZK proof for integrity of provided `address_next_index` and
        // `address_next_value`.
        next_address_proof: [u8; 128],
    ) -> Result<()> {
        process_update_address_merkle_tree(
            ctx,
            changelog_index,
            queue_index,
            address_next_index,
            address_next_value,
            low_address,
            low_address_next_value,
            low_address_proof,
            next_address_proof,
        )
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
    /// Can only be called from the merkle_tree_authority.
    pub fn initialize_concurrent_merkle_tree(
        ctx: Context<InitializeConcurrentMerkleTree>,
        index: u64,
        owner: Pubkey,
        delegate: Option<Pubkey>,
    ) -> Result<()> {
        process_initialize_concurrent_state_merkle_tree(ctx, index, owner, delegate)
    }

    pub fn insert_leaves_into_merkle_trees<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InsertTwoLeavesParallel<'info>>,
        leaves: Vec<[u8; 32]>,
    ) -> Result<()> {
        process_insert_leaves_into_merkle_trees(ctx, &leaves)
    }

    // TODO: add insert into merkle tree function that inserts multiple leaves into a single merkle tree
    pub fn initialize_indexed_array<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InitializeIndexedArrays<'info>>,
        index: u64,
        owner: Pubkey,
        delegate: Option<Pubkey>,
    ) -> Result<()> {
        process_initialize_indexed_array(ctx, index, owner, delegate)
    }

    pub fn insert_into_indexed_arrays<'a, 'b, 'c: 'info, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InsertIntoIndexedArrays<'info>>,
        elements: Vec<[u8; 32]>,
        low_element_indexes: Vec<u16>,
    ) -> Result<()> {
        process_insert_into_indexed_arrays(ctx, &elements, &low_element_indexes)
    }

    // TODO: insert into indexed array just insert into one array instead of possibly multiple

    // TODO: insert_from_indexed_array_into_merkle_tree ( to nullify transactions)
}
