use account_compression::{
    program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED, AddressMerkleTreeConfig,
    AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};
use anchor_lang::prelude::*;
use light_system_program::program::LightSystemProgram;

use crate::{
    errors::RegistryError,
    protocol_config::state::{ProtocolConfig, ProtocolConfigPda},
};

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
    pub protocol_config_pda: Account<'info, ProtocolConfigPda>,
    /// CHECK: (system program) new cpi context account.
    pub cpi_context_account: Option<AccountInfo<'info>>,
    pub light_system_program: Option<Program<'info, LightSystemProgram>>,
}

pub fn process_initialize_state_merkle_tree(
    ctx: &Context<InitializeMerkleTreeAndQueue>,
    bump: u8,
    index: u64,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    merkle_tree_config: StateMerkleTreeConfig,
    queue_config: NullifierQueueConfig,
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
        0,
    )
}

pub fn process_initialize_address_merkle_tree(
    ctx: Context<InitializeMerkleTreeAndQueue>,
    bump: u8,
    index: u64,
    program_owner: Option<Pubkey>,
    forester: Option<Pubkey>,
    merkle_tree_config: AddressMerkleTreeConfig,
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

pub fn process_initialize_cpi_context<'info>(
    bump: u8,
    fee_payer: AccountInfo<'info>,
    cpi_context_account: AccountInfo<'info>,
    associated_merkle_tree: AccountInfo<'info>,
    light_system_program: AccountInfo<'info>,
) -> Result<()> {
    let bump = &[bump];
    let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
    let signer_seeds = &[&seeds[..]];
    let accounts = light_system_program::cpi::accounts::InitializeCpiContextAccount {
        fee_payer,
        cpi_context_account,
        associated_merkle_tree,
    };
    let cpi_ctx = CpiContext::new_with_signer(light_system_program, accounts, signer_seeds);

    light_system_program::cpi::init_cpi_context_account(cpi_ctx)
}

pub fn check_cpi_context(
    account: AccountInfo<'_>,
    protocol_config: &ProtocolConfig,
) -> Result<u64> {
    let config_cpi_context_account_len = protocol_config.cpi_context_size as usize;
    if account.data_len() != config_cpi_context_account_len {
        msg!(
            "CPI context account data len: {}, expected: {}",
            account.data_len(),
            config_cpi_context_account_len
        );
        return err!(RegistryError::CpiContextAccountInvalidDataLen);
    }
    let rent = Rent::get()?;
    Ok(rent.minimum_balance(config_cpi_context_account_len))
}
