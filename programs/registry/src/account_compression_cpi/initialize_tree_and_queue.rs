use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, AddressMerkleTreeConfig,
    AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct InitializeMerkleTreeAndQueue<'info> {
    /// Anyone can create new trees just the fees cannot be set arbitrarily.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: (account compression program).
    #[account(mut)]
    pub queue: AccountInfo<'info>,
    /// CHECK: (account compression program) access control.
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK: (seed constraints) used to invoke account compression program via cpi.
    #[account(mut, seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    pub cpi_authority: AccountInfo<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
}

pub fn process_initialize_state_merkle_tree(
    ctx: Context<InitializeMerkleTreeAndQueue>,
    bump: u8,
    index: u64, // TODO: replace with counter from pda
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    merkle_tree_config: StateMerkleTreeConfig, // TODO: check config with protocol config
    queue_config: NullifierQueueConfig,
    additional_rent: u64,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::InitializeStateMerkleTreeAndNullifierQueue {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        nullifier_queue: ctx.accounts.queue.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.clone()),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::initialize_state_merkle_tree_and_nullifier_queue(
        cpi_ctx,
        index,
        program_owner,
        forester,
        merkle_tree_config,
        queue_config,
        additional_rent,
    )
}

pub fn process_initialize_address_merkle_tree(
    ctx: Context<InitializeMerkleTreeAndQueue>,
    bump: u8,
    index: u64, // TODO: replace with counter from pda
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    merkle_tree_config: AddressMerkleTreeConfig, // TODO: check config with protocol config
    queue_config: AddressQueueConfig,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = account_compression::cpi::accounts::InitializeAddressMerkleTreeAndQueue {
        authority: ctx.accounts.cpi_authority.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        queue: ctx.accounts.queue.to_account_info(),
        registered_program_pda: Some(ctx.accounts.registered_program_pda.clone()),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.account_compression_program.to_account_info(),
        accounts,
        signer_seeds,
    );

    account_compression::cpi::initialize_address_merkle_tree_and_queue(
        cpi_ctx,
        index,
        program_owner,
        forester,
        merkle_tree_config,
        queue_config,
    )
}
