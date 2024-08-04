#![allow(clippy::too_many_arguments)]
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use account_compression::{AddressMerkleTreeConfig, AddressQueueConfig};
use account_compression::{NullifierQueueConfig, StateMerkleTreeConfig};
use anchor_lang::prelude::*;

pub mod account_compression_cpi;
pub mod errors;
pub use crate::epoch::{finalize_registration::*, register_epoch::*, report_work::*};
pub use account_compression_cpi::{
    initialize_tree_and_queue::*, nullify::*, register_program::*, rollover_state_tree::*,
    update_address_tree::*,
};

pub use protocol_config::{initialize::*, update::*};
pub mod epoch;
pub mod protocol_config;
pub mod selection;
pub mod utils;
pub use selection::forester::*;

use protocol_config::state::ProtocolConfig;

#[cfg(not(target_os = "solana"))]
pub mod sdk;
declare_id!("7Z9Yuy3HkBCc2Wf3xzMGnz6qpV4n7ciwcoEMGKqhAnj1");

#[program]
pub mod light_registry {

    use anchor_lang::solana_program::pubkey::Pubkey;

    use super::*;

    pub fn initialize_governance_authority(
        ctx: Context<InitializeAuthority>,
        bump: u8,
        protocol_config: ProtocolConfig,
    ) -> Result<()> {
        ctx.accounts.authority_pda.authority = ctx.accounts.authority.key();
        ctx.accounts.authority_pda.bump = bump;
        ctx.accounts.authority_pda.config = protocol_config;
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
        ForesterEpochPda::check_forester_in_program(
            &mut ctx.accounts.registered_forester_pda,
            &ctx.accounts.authority.key(),
            &ctx.accounts.nullifier_queue.key(),
        )?;
        process_nullify(
            ctx,
            bump,
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
        ForesterEpochPda::check_forester_in_program(
            &mut ctx.accounts.registered_forester_pda,
            &ctx.accounts.authority.key(),
            &ctx.accounts.queue.key(),
        )?;
        process_update_address_merkle_tree(
            ctx,
            bump,
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
        ForesterEpochPda::check_forester_in_program(
            &mut ctx.accounts.registered_forester_pda,
            &ctx.accounts.authority.key(),
            &ctx.accounts.old_queue.key(),
        )?;
        process_rollover_address_merkle_tree_and_queue(ctx, bump)
    }

    pub fn rollover_state_merkle_tree_and_queue(
        ctx: Context<RolloverMerkleTreeAndQueue>,
        bump: u8,
    ) -> Result<()> {
        // TODO: specificy how to forest rolled over queues
        ForesterEpochPda::check_forester_in_program(
            &mut ctx.accounts.registered_forester_pda,
            &ctx.accounts.authority.key(),
            &ctx.accounts.old_queue.key(),
        )?;
        process_rollover_state_merkle_tree_and_queue(ctx, bump)
    }

    pub fn register_forester(
        ctx: Context<RegisterForester>,
        _bump: u8,
        authority: Pubkey,
        config: ForesterConfig,
    ) -> Result<()> {
        ctx.accounts.forester_pda.authority = authority;
        ctx.accounts.forester_pda.config = config;
        // TODO: remove once delegating is implemented
        ctx.accounts.forester_pda.active_stake_weight = 1;
        msg!(
            "registered forester: {:?}",
            ctx.accounts.forester_pda.authority
        );
        msg!("registered forester pda: {:?}", ctx.accounts.forester_pda);
        Ok(())
    }

    pub fn update_forester(ctx: Context<UpdateForester>, config: ForesterConfig) -> Result<()> {
        if let Some(authority) = ctx.accounts.new_authority.as_ref() {
            ctx.accounts.forester_pda.authority = authority.key();
        }
        ctx.accounts.forester_pda.config = config;
        msg!(
            "updated forester: {:?}",
            ctx.accounts.forester_pda.authority
        );
        msg!("updated forester pda: {:?}", ctx.accounts.forester_pda);
        Ok(())
    }

    /// Registers the forester for the epoch.
    /// 1. Only the forester can register herself for the epoch.
    /// 2. Protocol config is copied.
    /// 3. Epoch account is created if needed.
    pub fn register_forester_epoch<'info>(
        ctx: Context<'_, '_, '_, 'info, RegisterForesterEpoch<'info>>,
        epoch: u64,
    ) -> Result<()> {
        let protocol_config = ctx.accounts.protocol_config.config.clone();
        let current_solana_slot = anchor_lang::solana_program::clock::Clock::get()?.slot;
        // Init epoch account if not initialized
        let current_epoch = protocol_config.get_current_epoch(current_solana_slot);
        // TODO: check that epoch is in registration phase
        if current_epoch != epoch {
            return err!(errors::RegistryError::InvalidEpoch);
        }
        // Only init if not initialized
        if ctx.accounts.epoch_pda.registered_stake == 0 {
            (*ctx.accounts.epoch_pda).clone_from(&EpochPda {
                epoch: current_epoch,
                protocol_config: ctx.accounts.protocol_config.config.clone(),
                total_work: 0,
                registered_stake: 0,
            });
        }
        register_for_epoch_instruction(
            &ctx.accounts.authority.key(),
            &mut ctx.accounts.forester_pda,
            &mut ctx.accounts.forester_epoch_pda,
            &mut ctx.accounts.epoch_pda,
            current_solana_slot,
        )?;
        Ok(())
    }

    /// This transaction can be included as additional instruction in the first
    /// work instructions during the active phase.
    /// Registration Period must be over.
    /// TODO: introduce grace period between registration and before
    /// active phase starts, do I really need it or isn't it clear who gets the
    /// first slot the first sign up?
    pub fn finalize_registration<'info>(
        ctx: Context<'_, '_, '_, 'info, FinalizeRegistration<'info>>,
    ) -> Result<()> {
        let current_solana_slot = anchor_lang::solana_program::clock::Clock::get()?.slot;
        let current_epoch = ctx
            .accounts
            .epoch_pda
            .protocol_config
            .get_current_active_epoch(current_solana_slot)?;
        if current_epoch != ctx.accounts.epoch_pda.epoch
            || ctx.accounts.epoch_pda.epoch != ctx.accounts.forester_epoch_pda.epoch
        {
            return err!(errors::RegistryError::InvalidEpoch);
        }
        ctx.accounts.forester_epoch_pda.total_epoch_state_weight =
            Some(ctx.accounts.epoch_pda.registered_stake);
        ctx.accounts.forester_epoch_pda.finalize_counter += 1;
        // TODO: add limit for finalize counter to throw if exceeded
        // Is a safeguard so that noone can block parallelism
        Ok(())
    }

    pub fn update_forester_epoch_pda(
        ctx: Context<UpdateForesterEpochPda>,
        authority: Pubkey,
    ) -> Result<()> {
        ctx.accounts.forester_epoch_pda.authority = authority;
        Ok(())
    }

    pub fn report_work<'info>(ctx: Context<'_, '_, '_, 'info, ReportWork<'info>>) -> Result<()> {
        let current_solana_slot = anchor_lang::solana_program::clock::Clock::get()?.slot;
        ctx.accounts
            .epoch_pda
            .protocol_config
            .is_report_work_phase(current_solana_slot, ctx.accounts.epoch_pda.epoch)?;
        // TODO: unify epoch security checks
        if ctx.accounts.epoch_pda.epoch != ctx.accounts.forester_epoch_pda.epoch {
            return err!(errors::RegistryError::InvalidEpoch);
        }
        if ctx.accounts.forester_epoch_pda.has_reported_work {
            return err!(errors::RegistryError::ForesterAlreadyReportedWork);
        }
        ctx.accounts.epoch_pda.total_work += ctx.accounts.forester_epoch_pda.work_counter;
        ctx.accounts.forester_epoch_pda.has_reported_work = true;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialize_address_merkle_tree(
        ctx: Context<InitializeMerkleTreeAndQueue>,
        bump: u8,
        index: u64, // TODO: replace with counter from pda
        program_owner: Option<Pubkey>,
        merkle_tree_config: AddressMerkleTreeConfig, // TODO: check config with protocol config
        queue_config: AddressQueueConfig,
    ) -> Result<()> {
        process_initialize_address_merkle_tree(
            ctx,
            bump,
            index,
            program_owner,
            Some(crate::ID), // test value
            merkle_tree_config,
            queue_config,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialize_state_merkle_tree(
        ctx: Context<InitializeMerkleTreeAndQueue>,
        bump: u8,
        index: u64, // TODO: replace with counter from pda
        program_owner: Option<Pubkey>,
        merkle_tree_config: StateMerkleTreeConfig, // TODO: check config with protocol config
        queue_config: NullifierQueueConfig,
        additional_rent: u64,
    ) -> Result<()> {
        process_initialize_state_merkle_tree(
            ctx,
            bump,
            index,
            program_owner,
            Some(crate::ID), // test value
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
