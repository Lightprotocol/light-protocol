#![allow(clippy::too_many_arguments)]
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use account_compression::{program::AccountCompression, state::GroupAuthority};
use account_compression::{AddressMerkleTreeConfig, AddressQueueConfig};
use account_compression::{NullifierQueueConfig, StateMerkleTreeConfig};
use anchor_lang::prelude::*;

pub mod forester;
pub use forester::*;
#[cfg(not(target_os = "solana"))]
pub mod sdk;

declare_id!("7Z9Yuy3HkBCc2Wf3xzMGnz6qpV4n7ciwcoEMGKqhAnj1");

#[error_code]
pub enum RegistryError {
    #[msg("InvalidForester")]
    InvalidForester,
}

#[constant]
pub const AUTHORITY_PDA_SEED: &[u8] = b"authority";

#[program]
pub mod light_registry {

    use anchor_lang::solana_program::pubkey::Pubkey;

    use super::*;

    pub fn initialize_governance_authority(
        ctx: Context<InitializeAuthority>,
        authority: Pubkey,
        rewards: Vec<u64>,
        bump: u8,
    ) -> Result<()> {
        ctx.accounts.authority_pda.authority = authority;
        ctx.accounts.authority_pda.bump = bump;
        ctx.accounts.authority_pda.rewards = rewards;
        ctx.accounts.authority_pda.epoch = 0;
        ctx.accounts.authority_pda.epoch_length = u64::MAX;
        Ok(())
    }

    pub fn update_governance_authority(
        ctx: Context<UpdateAuthority>,
        bump: u8,
        new_authority: Pubkey,
    ) -> Result<()> {
        ctx.accounts.authority_pda.authority = new_authority;
        ctx.accounts.authority_pda.bump = bump;
        Ok(())
    }

    pub fn register_system_program(ctx: Context<RegisteredProgram>, bump: u8) -> Result<()> {
        let bump = &[bump];
        let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
        let signer_seeds = &[&seeds[..]];

        let accounts = account_compression::cpi::accounts::RegisterProgramToGroup {
            authority: ctx.accounts.cpi_authority.to_account_info(),
            program_to_be_registered: ctx.accounts.program_to_be_registered.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
            registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
            group_authority_pda: ctx.accounts.group_pda.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        account_compression::cpi::register_program_to_group(cpi_ctx)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn nullify(
        ctx: Context<NullifyLeaves>,
        bump: u8,
        change_log_indices: Vec<u64>,
        leaves_queue_indices: Vec<u16>,
        indices: Vec<u64>,
        proofs: Vec<Vec<[u8; 32]>>,
    ) -> Result<()> {
        check_forester(
            &mut ctx.accounts.registered_forester_pda,
            &ctx.accounts.authority.key(),
        )?;
        let bump = &[bump];
        let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
        let signer_seeds = &[&seeds[..]];
        let accounts = account_compression::cpi::accounts::NullifyLeaves {
            authority: ctx.accounts.cpi_authority.to_account_info(),
            registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
            log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
            merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
            nullifier_queue: ctx.accounts.nullifier_queue.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        account_compression::cpi::nullify_leaves(
            cpi_ctx,
            change_log_indices,
            leaves_queue_indices,
            indices,
            proofs,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_address_merkle_tree(
        ctx: Context<UpdateAddressMerkleTree>,
        bump: u8,
        changelog_index: u16,
        indexed_changelog_index: u16,
        value: u16,
        low_address_index: u64,
        low_address_value: [u8; 32],
        low_address_next_index: u64,
        low_address_next_value: [u8; 32],
        low_address_proof: [[u8; 32]; 16],
    ) -> Result<()> {
        check_forester(
            &mut ctx.accounts.registered_forester_pda,
            &ctx.accounts.authority.key(),
        )?;
        let bump = &[bump];
        let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
        let signer_seeds = &[&seeds[..]];

        let accounts = account_compression::cpi::accounts::UpdateAddressMerkleTree {
            authority: ctx.accounts.cpi_authority.to_account_info(),
            registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
            log_wrapper: ctx.accounts.log_wrapper.to_account_info(),
            queue: ctx.accounts.queue.to_account_info(),
            merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        account_compression::cpi::update_address_merkle_tree(
            cpi_ctx,
            changelog_index,
            indexed_changelog_index,
            value,
            low_address_index,
            low_address_value,
            low_address_next_index,
            low_address_next_value,
            low_address_proof,
        )
    }

    pub fn rollover_address_merkle_tree_and_queue(
        ctx: Context<RolloverMerkleTreeAndQueue>,
        bump: u8,
    ) -> Result<()> {
        check_forester(
            &mut ctx.accounts.registered_forester_pda,
            &ctx.accounts.authority.key(),
        )?;
        let bump = &[bump];

        let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
        let signer_seeds = &[&seeds[..]];

        let accounts = account_compression::cpi::accounts::RolloverAddressMerkleTreeAndQueue {
            fee_payer: ctx.accounts.authority.to_account_info(),
            authority: ctx.accounts.cpi_authority.to_account_info(),
            registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
            new_address_merkle_tree: ctx.accounts.new_merkle_tree.to_account_info(),
            new_queue: ctx.accounts.new_queue.to_account_info(),
            old_address_merkle_tree: ctx.accounts.old_merkle_tree.to_account_info(),
            old_queue: ctx.accounts.old_queue.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        account_compression::cpi::rollover_address_merkle_tree_and_queue(cpi_ctx)
    }

    pub fn rollover_state_merkle_tree_and_queue(
        ctx: Context<RolloverMerkleTreeAndQueue>,
        bump: u8,
    ) -> Result<()> {
        check_forester(
            &mut ctx.accounts.registered_forester_pda,
            &ctx.accounts.authority.key(),
        )?;
        let bump = &[bump];

        let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
        let signer_seeds = &[&seeds[..]];

        let accounts =
            account_compression::cpi::accounts::RolloverStateMerkleTreeAndNullifierQueue {
                fee_payer: ctx.accounts.authority.to_account_info(),
                authority: ctx.accounts.cpi_authority.to_account_info(),
                registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
                new_state_merkle_tree: ctx.accounts.new_merkle_tree.to_account_info(),
                new_nullifier_queue: ctx.accounts.new_queue.to_account_info(),
                old_state_merkle_tree: ctx.accounts.old_merkle_tree.to_account_info(),
                old_nullifier_queue: ctx.accounts.old_queue.to_account_info(),
            };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        account_compression::cpi::rollover_state_merkle_tree_and_nullifier_queue(cpi_ctx)
    }

    pub fn register_forester(
        ctx: Context<RegisterForester>,
        _bump: u8,
        authority: Pubkey,
    ) -> Result<()> {
        ctx.accounts.forester_epoch_pda.authority = authority;
        ctx.accounts.forester_epoch_pda.epoch_start = 0;
        ctx.accounts.forester_epoch_pda.epoch_end = u64::MAX;
        msg!(
            "registered forester: {:?}",
            ctx.accounts.forester_epoch_pda.authority
        );
        msg!(
            "registered forester pda: {:?}",
            ctx.accounts.forester_epoch_pda
        );
        Ok(())
    }

    pub fn update_forester_epoch_pda(
        ctx: Context<UpdateForesterEpochPda>,
        authority: Pubkey,
    ) -> Result<()> {
        ctx.accounts.forester_epoch_pda.authority = authority;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialize_address_merkle_tree(
        ctx: Context<InitializeAddressMerkleTreeAndQueue>,
        bump: u8,
        index: u64, // TODO: replace with counter from pda
        program_owner: Option<Pubkey>,
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
            merkle_tree_config,
            queue_config,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialize_state_merkle_tree(
        ctx: Context<InitializeAddressMerkleTreeAndQueue>,
        bump: u8,
        index: u64, // TODO: replace with counter from pda
        program_owner: Option<Pubkey>,
        merkle_tree_config: StateMerkleTreeConfig, // TODO: check config with protocol config
        queue_config: NullifierQueueConfig,
        additional_rent: u64,
    ) -> Result<()> {
        let bump = &[bump];
        let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
        let signer_seeds = &[&seeds[..]];
        let accounts =
            account_compression::cpi::accounts::InitializeStateMerkleTreeAndNullifierQueue {
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
            merkle_tree_config,
            queue_config,
            additional_rent,
        )
    }

    // TODO: update rewards field
    // signer is light governance authority

    // TODO: sync rewards
    // signer is registered relayer
    // sync rewards field with Light Governance Authority rewards field

    // TODO: add register relayer
    // signer is light governance authority
    // creates a registered relayer pda which is derived from the relayer
    // pubkey, with fields: signer_pubkey, points_counter, rewards: Vec<u64>,
    // last_rewards_sync

    // TODO: deregister relayer
    // signer is light governance authority

    // TODO: update registered relayer
    // signer is registered relayer
    // update the relayer signer pubkey in the pda

    // TODO: add rollover Merkle tree with rewards
    // signer is registered relayer
    // cpi to account compression program rollover Merkle tree
    // increment points in registered relayer account

    // TODO: add rollover lookup table with rewards
    // signer is registered relayer
    // cpi to account compression program rollover lookup table
    // increment points in registered relayer account

    // TODO: add nullify compressed_account with rewards
    // signer is registered relayer
    // cpi to account compression program nullify compressed_account
    // increment points in registered relayer account
}

#[derive(Accounts)]
pub struct InitializeAddressMerkleTreeAndQueue<'info> {
    /// Anyone can create new trees just the fees cannot be set arbitrarily.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub queue: AccountInfo<'info>,
    /// CHECK:
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    cpi_authority: AccountInfo<'info>,
    account_compression_program: Program<'info, AccountCompression>,
}

#[derive(Accounts)]
pub struct InitializeStateMerkleTreeAndQueue<'info> {
    /// Anyone can create new trees just the fees cannot be set arbitrarily.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub queue: AccountInfo<'info>,
    /// CHECK:
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    cpi_authority: AccountInfo<'info>,
    account_compression_program: Program<'info, AccountCompression>,
}

#[derive(Debug)]
#[account]
pub struct LightGovernanceAuthority {
    pub authority: Pubkey,
    pub bump: u8,
    pub epoch: u64,
    pub epoch_length: u64,
    pub _padding: [u8; 7],
    pub rewards: Vec<u64>, // initializing with storage for 8 u64s TODO: add instruction to resize
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializeAuthority<'info> {
    // TODO: add check that this is upgrade authority
    #[account(mut)]
    authority: Signer<'info>,
    /// CHECK:
    #[account(init, seeds = [AUTHORITY_PDA_SEED], bump, space = 8 + 32 + 8 + 8 * 8, payer = authority)]
    authority_pda: Account<'info, LightGovernanceAuthority>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct UpdateAuthority<'info> {
    #[account(mut, constraint = authority.key() == authority_pda.authority)]
    authority: Signer<'info>,
    /// CHECK:
    #[account(mut, seeds = [AUTHORITY_PDA_SEED], bump)]
    authority_pda: Account<'info, LightGovernanceAuthority>,
}

#[derive(Accounts)]
pub struct RegisteredProgram<'info> {
    #[account(mut, constraint = authority.key() == authority_pda.authority)]
    authority: Signer<'info>,
    /// CHECK:
    #[account(mut, seeds = [AUTHORITY_PDA_SEED], bump)]
    authority_pda: Account<'info, LightGovernanceAuthority>,
    /// CHECK: this is
    #[account(mut, seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    cpi_authority: AccountInfo<'info>,
    #[account(mut)]
    group_pda: Account<'info, GroupAuthority>,
    account_compression_program: Program<'info, AccountCompression>,
    system_program: Program<'info, System>,
    /// CHECK:
    registered_program_pda: AccountInfo<'info>,
    /// CHECK: is checked in the account compression program.
    program_to_be_registered: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct NullifyLeaves<'info> {
    /// CHECK:
    #[account(mut)]
    pub registered_forester_pda: Account<'info, ForesterEpoch>,
    /// CHECK: unchecked for now logic that regulates forester access is yet to be added.
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    cpi_authority: AccountInfo<'info>,
    /// CHECK:
    #[account(
        seeds = [&crate::ID.to_bytes()], bump, seeds::program = &account_compression::ID,
        )]
    pub registered_program_pda:
        Account<'info, account_compression::instructions::register_program::RegisteredProgram>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
    /// CHECK: in account compression program
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: in account compression program
    #[account(mut)]
    pub nullifier_queue: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RolloverMerkleTreeAndQueue<'info> {
    /// CHECK:
    #[account(mut)]
    pub registered_forester_pda: Account<'info, ForesterEpoch>,
    /// CHECK: unchecked for now logic that regulates forester access is yet to be added.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    cpi_authority: AccountInfo<'info>,
    /// CHECK:
    #[account(
        seeds = [&crate::ID.to_bytes()], bump, seeds::program = &account_compression::ID,
        )]
    pub registered_program_pda:
        Account<'info, account_compression::instructions::register_program::RegisteredProgram>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK:
    #[account(zero)]
    pub new_merkle_tree: AccountInfo<'info>,
    /// CHECK:
    #[account(zero)]
    pub new_queue: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub old_merkle_tree: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub old_queue: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UpdateAddressMerkleTree<'info> {
    /// CHECK:
    #[account(mut)]
    pub registered_forester_pda: Account<'info, ForesterEpoch>,
    /// CHECK: unchecked for now logic that regulates forester access is yet to be added.
    pub authority: Signer<'info>,
    /// CHECK:
    #[account(seeds = [CPI_AUTHORITY_PDA_SEED], bump)]
    cpi_authority: AccountInfo<'info>,
    /// CHECK:
    #[account(
        seeds = [&crate::ID.to_bytes()], bump, seeds::program = &account_compression::ID,
        )]
    pub registered_program_pda:
        Account<'info, account_compression::instructions::register_program::RegisteredProgram>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK: in account compression program
    #[account(mut)]
    pub queue: AccountInfo<'info>,
    /// CHECK: in account compression program
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK: when emitting event.
    pub log_wrapper: UncheckedAccount<'info>,
}
