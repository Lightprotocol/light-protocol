use anchor_lang::prelude::*;

declare_id!("JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "psp_account_compression",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol-program/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol-program/program_merkle_tree"
}

pub mod instructions;
pub use instructions::*;
pub mod state;
pub use state::*;
pub mod errors;
pub mod utils;

pub mod config_accounts;
pub use config_accounts::*;

#[program]
pub mod psp_account_compression {
    use super::*;

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

    // pub fn register_program_to_group<'info>(
    //     ctx: Context<'_, '_, '_, 'info, RegisterProgramToGroup<'info>>,
    //     program_id: Pubkey,
    // ) -> Result<()> {
    //     process_register_program_to_group(ctx, program_id)
    // }

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

    pub fn insert_leaves_into_merkle_trees<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InsertTwoLeavesParallel<'info>>,
        leaves: Vec<[u8; 32]>,
    ) -> Result<()> {
        process_insert_leaves_into_merkle_trees(ctx, &leaves)
    }

    // TODO: add insert into merkle tree function that inserts multiple leaves into a single merkle tree
    pub fn initialize_indexed_array<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeIndexedArrays<'info>>,
        index: u64,
        owner: Pubkey,
        delegate: Option<Pubkey>,
    ) -> Result<()> {
        process_initialize_indexed_array(ctx, index, owner, delegate)
    }

    pub fn insert_into_indexed_arrays<'info>(
        ctx: Context<'_, '_, '_, 'info, InsertIntoIndexedArrays<'info>>,
        elements: Vec<[u8; 32]>,
        low_element_indexes: Vec<u16>,
    ) -> Result<()> {
        process_insert_into_indexed_arrays(ctx, &elements, &low_element_indexes)
    }

    // TODO: insert into indexed array just insert into one array instead of possibly multiple

    // TODO: insert_from_indexed_array_into_merkle_tree ( to nullify transactions)
}
