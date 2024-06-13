#![allow(clippy::too_many_arguments)]
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use account_compression::{AddressMerkleTreeConfig, AddressQueueConfig};
use account_compression::{NullifierQueueConfig, StateMerkleTreeConfig};
use anchor_lang::prelude::*;

pub mod account_compression_cpi;
pub mod errors;
pub use crate::epoch::{
    claim_forester_instruction::*, finalize_registration::*, register_epoch::*, report_work::*,
    sync_delegate::process_sync_delegate_account, sync_delegate_instruction::*,
};
pub use account_compression_cpi::{
    initialize_tree_and_queue::*, nullify::*, register_program::*, rollover_state_tree::*,
    update_address_tree::*,
};

pub use protocol_config::{initialize::*, mint::*, update::*};
pub mod delegate;
pub mod epoch;
pub mod forester;
pub mod protocol_config;
pub mod utils;
use anchor_lang::solana_program::pubkey::Pubkey;
use delegate::deposit::{process_deposit_or_withdrawal, InputDelegateAccountWithPackedContext};
use delegate::process_delegate::process_delegate_or_undelegate;
pub use delegate::{delegate_instruction::*, deposit_instruction::*};
use delegate::{
    deposit::DelegateAccountWithPackedContext,
    process_cpi::{cpi_compressed_token_mint_to, get_cpi_signer_seeds},
};
use epoch::claim_forester::process_forester_claim_rewards;
use epoch::{
    claim_forester::CompressedForesterEpochAccountInput, sync_delegate::SyncDelegateTokenAccount,
};
pub use forester::state::*;
use light_compressed_token::process_transfer::InputTokenDataWithContext;
use light_system_program::sdk::compressed_account::PackedMerkleContext;
use light_system_program::{invoke::processor::CompressedProof, sdk::CompressedCpiContext};
use protocol_config::state::ProtocolConfig;

#[cfg(not(target_os = "solana"))]
pub mod sdk;
declare_id!("7Z9Yuy3HkBCc2Wf3xzMGnz6qpV4n7ciwcoEMGKqhAnj1");

#[program]
pub mod light_registry {

    use super::*;

    pub fn initialize_governance_authority(
        ctx: Context<InitializeAuthority>,
        bump: u8,
        protocol_config: ProtocolConfig,
    ) -> Result<()> {
        if protocol_config.mint != ctx.accounts.mint.key()
            || ctx.accounts.mint.mint_authority.unwrap() != ctx.accounts.cpi_authority.key()
        {
            return err!(errors::RegistryError::InvalidMint);
        }
        ctx.accounts.authority_pda.authority = ctx.accounts.authority.key();
        ctx.accounts.authority_pda.bump = bump;
        ctx.accounts.authority_pda.config = protocol_config;
        msg!("mint: {:?}", ctx.accounts.mint.key());
        Ok(())
    }

    // TODO: rename to update_protocol_config
    pub fn update_governance_authority(
        ctx: Context<UpdateAuthority>,
        _bump: u8,
        new_authority: Pubkey,
        new_config: ProtocolConfig,
    ) -> Result<()> {
        ctx.accounts.authority_pda.authority = new_authority;
        // ctx.accounts.authority_pda.bump = bump;
        // mint cannot be updated
        if ctx.accounts.authority_pda.config.mint != new_config.mint {
            return err!(errors::RegistryError::InvalidMint);
        }
        // forester registration guarded can only be disabled
        if !ctx
            .accounts
            .authority_pda
            .config
            .forester_registration_guarded
            && new_config.forester_registration_guarded
        {
            return err!(errors::RegistryError::InvalidProtocolConfigUpdate);
        }
        Ok(())
    }

    pub fn mint<'info>(
        ctx: Context<'_, '_, '_, 'info, Mint<'info>>,
        amounts: Vec<u64>,
        recipients: Vec<Pubkey>,
    ) -> Result<()> {
        cpi_compressed_token_mint_to(
            &ctx,
            recipients,
            amounts,
            get_cpi_signer_seeds(),
            ctx.accounts.merkle_tree.to_account_info(),
        )
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
        config: ForesterConfig,
    ) -> Result<()> {
        if ctx.accounts.protocol_config_pda.authority != ctx.accounts.signer.key()
            && ctx
                .accounts
                .protocol_config_pda
                .config
                .forester_registration_guarded
        {
            return err!(errors::RegistryError::InvalidAuthority);
        }
        ctx.accounts.forester_pda.authority = ctx.accounts.authority.key();
        ctx.accounts.forester_pda.config = config;
        // // TODO: remove once delegating is implemented
        // if ctx
        //     .accounts
        //     .protocol_config_pda
        //     .config
        //     .forester_registration_guarded
        // {
        //     ctx.accounts.forester_pda.active_stake_weight = 1;
        // }
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
        let protocol_config = ctx.accounts.protocol_config.config;
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
                protocol_config: ctx.accounts.protocol_config.config,
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
            merkle_tree_config,
            queue_config,
            additional_rent,
        )
    }

    pub fn deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, DepositOrWithdrawInstruction<'info>>,
        salt: u64,
        delegate_account: Option<InputDelegateAccountWithPackedContext>,
        deposit_amount: u64,
        input_compressed_token_accounts: Vec<InputTokenDataWithContext>,
        input_escrow_token_account: Option<InputTokenDataWithContext>,
        escrow_token_account_merkle_tree_index: u8,
        change_compressed_account_merkle_tree_index: u8,
        output_delegate_compressed_account_merkle_tree_index: u8,
        proof: CompressedProof,
        cpi_context: CompressedCpiContext,
    ) -> Result<()> {
        process_deposit_or_withdrawal::<true>(
            ctx,
            salt,
            proof,
            cpi_context,
            delegate_account,
            deposit_amount,
            input_compressed_token_accounts,
            input_escrow_token_account,
            escrow_token_account_merkle_tree_index,
            change_compressed_account_merkle_tree_index,
            output_delegate_compressed_account_merkle_tree_index,
        )
    }

    pub fn withdrawal<'info>(
        ctx: Context<'_, '_, '_, 'info, DepositOrWithdrawInstruction<'info>>,
        salt: u64,
        proof: CompressedProof,
        cpi_context: CompressedCpiContext,
        delegate_account: InputDelegateAccountWithPackedContext,
        withdrawal_amount: u64,
        input_escrow_token_account: InputTokenDataWithContext,
        escrow_token_account_merkle_tree_index: u8,
        change_compressed_account_merkle_tree_index: u8,
        output_delegate_compressed_account_merkle_tree_index: u8,
    ) -> Result<()> {
        process_deposit_or_withdrawal::<false>(
            ctx,
            salt,
            proof,
            cpi_context,
            Some(delegate_account),
            withdrawal_amount,
            Vec::new(),
            Some(input_escrow_token_account),
            escrow_token_account_merkle_tree_index,
            change_compressed_account_merkle_tree_index,
            output_delegate_compressed_account_merkle_tree_index,
        )
    }

    pub fn delegate<'info>(
        ctx: Context<'_, '_, '_, 'info, DelegatetOrUndelegateInstruction<'info>>,
        proof: CompressedProof,
        delegate_account: DelegateAccountWithPackedContext,
        delegate_amount: u64,
        no_sync: bool,
    ) -> Result<()> {
        process_delegate_or_undelegate::<true>(
            ctx,
            proof,
            delegate_account,
            delegate_amount,
            no_sync,
        )
    }

    pub fn undelegate<'info>(
        ctx: Context<'_, '_, '_, 'info, DelegatetOrUndelegateInstruction<'info>>,
        proof: CompressedProof,
        delegate_account: DelegateAccountWithPackedContext,
        delegate_amount: u64,
        no_sync: bool,
    ) -> Result<()> {
        process_delegate_or_undelegate::<false>(
            ctx,
            proof,
            delegate_account,
            delegate_amount,
            no_sync,
        )
    }

    pub fn claim_forester_rewards<'info>(
        ctx: Context<'_, '_, '_, 'info, ClaimForesterInstruction<'info>>,
    ) -> Result<()> {
        process_forester_claim_rewards(ctx)
    }

    pub fn sync_delegate<'info>(
        ctx: Context<'_, '_, '_, 'info, SyncDelegateInstruction<'info>>,
        _salt: u64,
        delegate_account: DelegateAccountWithPackedContext,
        previous_hash: [u8; 32],
        forester_pda_pubkey: Pubkey,
        compressed_forester_epoch_pdas: Vec<CompressedForesterEpochAccountInput>,
        last_account_root_index: u16,
        last_account_merkle_context: PackedMerkleContext,
        inclusion_proof: CompressedProof,
        sync_delegate_token_account: Option<SyncDelegateTokenAccount>,
        input_escrow_token_account: Option<InputTokenDataWithContext>,
        output_token_account_merkle_tree_index: u8,
    ) -> Result<()> {
        process_sync_delegate_account(
            ctx,
            delegate_account,
            previous_hash,
            forester_pda_pubkey,
            compressed_forester_epoch_pdas,
            last_account_root_index,
            last_account_merkle_context,
            inclusion_proof,
            sync_delegate_token_account,
            input_escrow_token_account,
            output_token_account_merkle_tree_index,
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

    // TODO: add rollover lookup table with rewards
    // signer is registered relayer
    // cpi to account compression program rollover lookup table
    // increment points in registered relayer account
}
