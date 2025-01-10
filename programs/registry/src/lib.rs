#![allow(clippy::too_many_arguments)]
use account_compression::{
    utils::constants::CPI_AUTHORITY_PDA_SEED, AddressMerkleTreeConfig, AddressQueueConfig,
    NullifierQueueConfig, StateMerkleTreeConfig,
};
use anchor_lang::prelude::*;
use light_merkle_tree_metadata::merkle_tree::MerkleTreeMetadata;

pub mod account_compression_cpi;
pub mod errors;
pub use account_compression_cpi::{
    batch_append::*, batch_nullify::*, batch_update_address_tree::*,
    initialize_batched_address_tree::*, initialize_batched_state_tree::*,
    initialize_tree_and_queue::*, migrate_state::*, nullify::*, register_program::*,
    rollover_batched_address_tree::*, rollover_batched_state_tree::*, rollover_state_tree::*,
    update_address_tree::*,
};
pub use protocol_config::{initialize::*, update::*};

pub use crate::epoch::{finalize_registration::*, register_epoch::*, report_work::*};
pub mod constants;
pub mod epoch;
pub mod protocol_config;
pub mod selection;
pub mod utils;
use account_compression::MigrateLeafParams;
use anchor_lang::solana_program::pubkey::Pubkey;
use errors::RegistryError;
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use protocol_config::state::ProtocolConfig;
pub use selection::forester::*;
#[cfg(not(target_os = "solana"))]
pub mod sdk;

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "light-registry",
    project_url: "lightprotocol.com",
    contacts: "email:security@lightprotocol.com",
    policy: "https://github.com/Lightprotocol/light-protocol/blob/main/SECURITY.md",
    source_code: "https://github.com/Lightprotocol/light-protocol"
}

declare_id!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX");

#[program]
pub mod light_registry {

    use constants::DEFAULT_WORK_V1;

    use super::*;

    /// Initializes the protocol config pda. Can only be called once by the
    /// program account keypair.
    pub fn initialize_protocol_config(
        ctx: Context<InitializeProtocolConfig>,
        bump: u8,
        protocol_config: ProtocolConfig,
    ) -> Result<()> {
        ctx.accounts.protocol_config_pda.authority = ctx.accounts.authority.key();
        ctx.accounts.protocol_config_pda.bump = bump;
        check_protocol_config(protocol_config)?;
        ctx.accounts.protocol_config_pda.config = protocol_config;
        Ok(())
    }

    pub fn update_protocol_config(
        ctx: Context<UpdateProtocolConfig>,
        protocol_config: Option<ProtocolConfig>,
    ) -> Result<()> {
        if let Some(new_authority) = ctx.accounts.new_authority.as_ref() {
            ctx.accounts.protocol_config_pda.authority = new_authority.key();
        }
        if let Some(protocol_config) = protocol_config {
            if protocol_config.genesis_slot != ctx.accounts.protocol_config_pda.config.genesis_slot
            {
                msg!("Genesis slot cannot be changed.");
                return err!(RegistryError::InvalidConfigUpdate);
            }
            if protocol_config.active_phase_length
                != ctx.accounts.protocol_config_pda.config.active_phase_length
            {
                msg!(
                    "Active phase length must not be changed, otherwise epochs will repeat {} {}.",
                    protocol_config.active_phase_length,
                    ctx.accounts.protocol_config_pda.config.active_phase_length
                );
                return err!(RegistryError::InvalidConfigUpdate);
            }
            check_protocol_config(protocol_config)?;
            ctx.accounts.protocol_config_pda.config = protocol_config;
        }
        Ok(())
    }

    pub fn register_system_program(ctx: Context<RegisterProgram>, bump: u8) -> Result<()> {
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

    pub fn deregister_system_program(ctx: Context<DeregisterProgram>, bump: u8) -> Result<()> {
        let bump = &[bump];
        let seeds = [CPI_AUTHORITY_PDA_SEED, bump];
        let signer_seeds = &[&seeds[..]];

        let accounts = account_compression::cpi::accounts::DeregisterProgram {
            authority: ctx.accounts.cpi_authority.to_account_info(),
            registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
            group_authority_pda: ctx.accounts.group_pda.to_account_info(),
            close_recipient: ctx.accounts.authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        account_compression::cpi::deregister_program(cpi_ctx)
    }

    pub fn register_forester(
        ctx: Context<RegisterForester>,
        _bump: u8,
        authority: Pubkey,
        config: ForesterConfig,
        weight: Option<u64>,
    ) -> Result<()> {
        ctx.accounts.forester_pda.authority = authority;
        ctx.accounts.forester_pda.config = config;

        if let Some(weight) = weight {
            ctx.accounts.forester_pda.active_weight = weight;
        }
        Ok(())
    }

    pub fn update_forester_pda(
        ctx: Context<UpdateForesterPda>,
        config: Option<ForesterConfig>,
    ) -> Result<()> {
        if let Some(authority) = ctx.accounts.new_authority.as_ref() {
            ctx.accounts.forester_pda.authority = authority.key();
        }
        if let Some(config) = config {
            ctx.accounts.forester_pda.config = config;
        }
        Ok(())
    }

    pub fn update_forester_pda_weight(
        ctx: Context<UpdateForesterPdaWeight>,
        new_weight: u64,
    ) -> Result<()> {
        ctx.accounts.forester_pda.active_weight = new_weight;
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
        // Only init if not initialized
        if ctx.accounts.epoch_pda.registered_weight == 0 {
            (*ctx.accounts.epoch_pda).clone_from(&EpochPda {
                epoch,
                protocol_config: ctx.accounts.protocol_config.config,
                total_work: 0,
                registered_weight: 0,
            });
        }
        let current_solana_slot = anchor_lang::solana_program::clock::Clock::get()?.slot;
        // Init epoch account if not initialized
        let current_epoch = ctx
            .accounts
            .epoch_pda
            .protocol_config
            .get_latest_register_epoch(current_solana_slot)?;

        if current_epoch != epoch {
            return err!(RegistryError::InvalidEpoch);
        }
        // check that epoch is in registration phase is in process_register_for_epoch
        process_register_for_epoch(
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
    pub fn finalize_registration<'info>(
        ctx: Context<'_, '_, '_, 'info, FinalizeRegistration<'info>>,
    ) -> Result<()> {
        let current_solana_slot = anchor_lang::solana_program::clock::Clock::get()?.slot;
        let current_active_epoch = ctx
            .accounts
            .epoch_pda
            .protocol_config
            .get_current_active_epoch(current_solana_slot)?;
        if current_active_epoch != ctx.accounts.epoch_pda.epoch {
            return err!(RegistryError::InvalidEpoch);
        }
        ctx.accounts.forester_epoch_pda.total_epoch_weight =
            Some(ctx.accounts.epoch_pda.registered_weight);
        ctx.accounts.forester_epoch_pda.finalize_counter += 1;
        // Check limit for finalize counter to throw if exceeded
        // Is a safeguard so that noone can block parallelism.
        // This instruction can be passed with nullify instructions, to prevent
        // read locking the epoch account for more than X transactions limit
        // the number of syncs without failing the tx to X
        if ctx.accounts.forester_epoch_pda.finalize_counter
            > ctx
                .accounts
                .forester_epoch_pda
                .protocol_config
                .finalize_counter_limit
        {
            return err!(RegistryError::FinalizeCounterExceeded);
        }

        Ok(())
    }

    pub fn report_work<'info>(ctx: Context<'_, '_, '_, 'info, ReportWork<'info>>) -> Result<()> {
        let current_solana_slot = anchor_lang::solana_program::clock::Clock::get()?.slot;
        ctx.accounts
            .epoch_pda
            .protocol_config
            .is_report_work_phase(current_solana_slot, ctx.accounts.epoch_pda.epoch)?;
        if ctx.accounts.epoch_pda.epoch != ctx.accounts.forester_epoch_pda.epoch {
            return err!(RegistryError::InvalidEpoch);
        }
        if ctx.accounts.forester_epoch_pda.has_reported_work {
            return err!(RegistryError::ForesterAlreadyReportedWork);
        }
        ctx.accounts.epoch_pda.total_work += ctx.accounts.forester_epoch_pda.work_counter;
        ctx.accounts.forester_epoch_pda.has_reported_work = true;
        Ok(())
    }

    pub fn initialize_address_merkle_tree(
        ctx: Context<InitializeMerkleTreeAndQueue>,
        bump: u8,
        program_owner: Option<Pubkey>,
        forester: Option<Pubkey>,
        merkle_tree_config: AddressMerkleTreeConfig,
        queue_config: AddressQueueConfig,
    ) -> Result<()> {
        // The network fee must be either zero or the same as the protocol config.
        // Only trees with a network fee will be serviced by light foresters.
        if let Some(network_fee) = merkle_tree_config.network_fee {
            if network_fee != ctx.accounts.protocol_config_pda.config.network_fee {
                return err!(RegistryError::InvalidNetworkFee);
            }
            if forester.is_some() {
                msg!("Forester pubkey must not be defined for trees serviced by light foresters.");
                return err!(RegistryError::ForesterDefined);
            }
        } else if forester.is_none() {
            msg!("Forester pubkey required for trees without a network fee.");
            msg!("Trees without a network fee will not be serviced by light foresters.");
            return err!(RegistryError::ForesterUndefined);
        }
        // Unused parameter
        if queue_config.network_fee.is_some() {
            return err!(RegistryError::InvalidNetworkFee);
        }
        process_initialize_address_merkle_tree(
            ctx,
            bump,
            0,
            program_owner,
            forester,
            merkle_tree_config,
            queue_config,
        )
    }

    pub fn initialize_state_merkle_tree(
        ctx: Context<InitializeMerkleTreeAndQueue>,
        bump: u8,
        program_owner: Option<Pubkey>,
        forester: Option<Pubkey>,
        merkle_tree_config: StateMerkleTreeConfig,
        queue_config: NullifierQueueConfig,
    ) -> Result<()> {
        // The network fee must be either zero or the same as the protocol config.
        // Only trees with a network fee will be serviced by light foresters.
        if let Some(network_fee) = merkle_tree_config.network_fee {
            if network_fee != ctx.accounts.protocol_config_pda.config.network_fee {
                return err!(RegistryError::InvalidNetworkFee);
            }
        } else if forester.is_none() {
            msg!("Forester pubkey required for trees without a network fee.");
            msg!("Trees without a network fee will not be serviced by light foresters.");
            return err!(RegistryError::ForesterUndefined);
        }

        // Unused parameter
        if queue_config.network_fee.is_some() {
            return err!(RegistryError::InvalidNetworkFee);
        }
        check_cpi_context(
            ctx.accounts
                .cpi_context_account
                .as_ref()
                .unwrap()
                .to_account_info(),
            &ctx.accounts.protocol_config_pda.config,
        )?;
        process_initialize_state_merkle_tree(
            &ctx,
            bump,
            0,
            program_owner,
            forester,
            merkle_tree_config,
            queue_config,
        )?;

        process_initialize_cpi_context(
            bump,
            ctx.accounts.authority.to_account_info(),
            ctx.accounts
                .cpi_context_account
                .as_ref()
                .unwrap()
                .to_account_info(),
            ctx.accounts.merkle_tree.to_account_info(),
            ctx.accounts
                .light_system_program
                .as_ref()
                .unwrap()
                .to_account_info(),
        )
    }

    pub fn nullify<'info>(
        ctx: Context<'_, '_, '_, 'info, NullifyLeaves<'info>>,
        bump: u8,
        change_log_indices: Vec<u64>,
        leaves_queue_indices: Vec<u16>,
        indices: Vec<u64>,
        proofs: Vec<Vec<[u8; 32]>>,
    ) -> Result<()> {
        let metadata = ctx.accounts.merkle_tree.load()?.metadata;
        check_forester(
            &metadata,
            ctx.accounts.authority.key(),
            ctx.accounts.nullifier_queue.key(),
            &mut ctx.accounts.registered_forester_pda,
            DEFAULT_WORK_V1,
        )?;

        process_nullify(
            &ctx,
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
        let metadata = ctx.accounts.merkle_tree.load()?.metadata;

        check_forester(
            &metadata,
            ctx.accounts.authority.key(),
            ctx.accounts.queue.key(),
            &mut ctx.accounts.registered_forester_pda,
            DEFAULT_WORK_V1,
        )?;
        process_update_address_merkle_tree(
            &ctx,
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

    pub fn rollover_address_merkle_tree_and_queue<'info>(
        ctx: Context<'_, '_, '_, 'info, RolloverAddressMerkleTreeAndQueue<'info>>,
        bump: u8,
    ) -> Result<()> {
        let metadata = ctx.accounts.old_merkle_tree.load()?.metadata;
        check_forester(
            &metadata,
            ctx.accounts.authority.key(),
            ctx.accounts.old_queue.key(),
            &mut ctx.accounts.registered_forester_pda,
            DEFAULT_WORK_V1,
        )?;

        process_rollover_address_merkle_tree_and_queue(&ctx, bump)
    }

    pub fn rollover_state_merkle_tree_and_queue<'info>(
        ctx: Context<'_, '_, '_, 'info, RolloverStateMerkleTreeAndQueue<'info>>,
        bump: u8,
    ) -> Result<()> {
        let metadata = ctx.accounts.old_merkle_tree.load()?.metadata;
        check_forester(
            &metadata,
            ctx.accounts.authority.key(),
            ctx.accounts.old_queue.key(),
            &mut ctx.accounts.registered_forester_pda,
            DEFAULT_WORK_V1,
        )?;

        check_cpi_context(
            ctx.accounts.cpi_context_account.to_account_info(),
            &ctx.accounts.protocol_config_pda.config,
        )?;
        process_rollover_state_merkle_tree_and_queue(&ctx, bump)?;
        process_initialize_cpi_context(
            bump,
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.cpi_context_account.to_account_info(),
            ctx.accounts.new_merkle_tree.to_account_info(),
            ctx.accounts.light_system_program.to_account_info(),
        )
    }

    pub fn initialize_batched_state_merkle_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeBatchedStateMerkleTreeAndQueue<'info>>,
        bump: u8,
        params: Vec<u8>,
    ) -> Result<()> {
        let params = InitStateTreeAccountsInstructionData::try_from_slice(&params)?;
        if let Some(network_fee) = params.network_fee {
            if network_fee != ctx.accounts.protocol_config_pda.config.network_fee {
                return err!(RegistryError::InvalidNetworkFee);
            }
            if params.forester.is_some() {
                msg!("Forester pubkey must not be defined for trees serviced by light foresters.");
                return err!(RegistryError::ForesterDefined);
            }
        } else if params.forester.is_none() {
            msg!("Forester pubkey required for trees without a network fee.");
            msg!("Trees without a network fee will not be serviced by light foresters.");
            return err!(RegistryError::ForesterUndefined);
        }
        check_cpi_context(
            ctx.accounts.cpi_context_account.to_account_info(),
            &ctx.accounts.protocol_config_pda.config,
        )?;

        process_initialize_batched_state_merkle_tree(&ctx, bump, params.try_to_vec().unwrap())?;

        process_initialize_cpi_context(
            bump,
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.cpi_context_account.to_account_info(),
            ctx.accounts.merkle_tree.to_account_info(),
            ctx.accounts.light_system_program.to_account_info(),
        )
    }

    pub fn batch_nullify<'info>(
        ctx: Context<'_, '_, '_, 'info, BatchNullify<'info>>,
        bump: u8,
        data: Vec<u8>,
    ) -> Result<()> {
        {
            let account = BatchedMerkleTreeAccount::state_tree_from_account_info_mut(
                &ctx.accounts.merkle_tree,
            )
            .map_err(ProgramError::from)?;
            let metadata = account.get_metadata().metadata;
            check_forester(
                &metadata,
                ctx.accounts.authority.key(),
                ctx.accounts.merkle_tree.key(),
                &mut ctx.accounts.registered_forester_pda,
                account.get_metadata().queue_metadata.batch_size,
            )?;
        }
        process_batch_nullify(&ctx, bump, data)
    }

    pub fn batch_append<'info>(
        ctx: Context<'_, '_, '_, 'info, BatchAppend<'info>>,
        bump: u8,
        data: Vec<u8>,
    ) -> Result<()> {
        {
            let queue_account =
                BatchedQueueAccount::output_queue_from_account_info_mut(&ctx.accounts.output_queue)
                    .map_err(ProgramError::from)?;
            let merkle_tree = BatchedMerkleTreeAccount::state_tree_from_account_info_mut(
                &ctx.accounts.merkle_tree,
            )
            .map_err(ProgramError::from)?;
            let metadata = merkle_tree.get_metadata().metadata;
            check_forester(
                &metadata,
                ctx.accounts.authority.key(),
                ctx.accounts.merkle_tree.key(),
                &mut ctx.accounts.registered_forester_pda,
                queue_account.get_metadata().batch_metadata.batch_size,
            )?;
        }
        process_batch_append(&ctx, bump, data)
    }

    pub fn initialize_batched_address_merkle_tree(
        ctx: Context<InitializeBatchedAddressTree>,
        bump: u8,
        params: Vec<u8>,
    ) -> Result<()> {
        let params = InitAddressTreeAccountsInstructionData::try_from_slice(&params)?;
        if let Some(network_fee) = params.network_fee {
            if network_fee != ctx.accounts.protocol_config_pda.config.network_fee {
                return err!(RegistryError::InvalidNetworkFee);
            }
            if params.forester.is_some() {
                msg!("Forester pubkey must not be defined for trees serviced by light foresters.");
                return err!(RegistryError::ForesterDefined);
            }
        } else if params.forester.is_none() {
            msg!("Forester pubkey required for trees without a network fee.");
            msg!("Trees without a network fee will not be serviced by light foresters.");
            return err!(RegistryError::ForesterUndefined);
        }
        process_initialize_batched_address_merkle_tree(&ctx, bump, params.try_to_vec().unwrap())
    }

    pub fn batch_update_address_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, BatchUpdateAddressTree<'info>>,
        bump: u8,
        data: Vec<u8>,
    ) -> Result<()> {
        {
            let account = BatchedMerkleTreeAccount::address_tree_from_account_info_mut(
                &ctx.accounts.merkle_tree,
            )
            .map_err(ProgramError::from)?;
            let account = account.get_metadata();
            let metadata = account.metadata;
            check_forester(
                &metadata,
                ctx.accounts.authority.key(),
                ctx.accounts.merkle_tree.key(),
                &mut ctx.accounts.registered_forester_pda,
                account.queue_metadata.batch_size,
            )?;
        }
        process_batch_update_address_tree(&ctx, bump, data)
    }

    pub fn rollover_batch_address_merkle_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, RolloverBatchAddressMerkleTree<'info>>,
        bump: u8,
    ) -> Result<()> {
        let account = BatchedMerkleTreeAccount::address_tree_from_account_info_mut(
            &ctx.accounts.old_address_merkle_tree,
        )
        .map_err(ProgramError::from)?;
        check_forester(
            &account.get_metadata().metadata,
            ctx.accounts.authority.key(),
            ctx.accounts.old_address_merkle_tree.key(),
            &mut ctx.accounts.registered_forester_pda,
            DEFAULT_WORK_V1,
        )?;
        process_rollover_batch_address_merkle_tree(&ctx, bump)
    }

    pub fn rollover_batch_state_merkle_tree<'info>(
        ctx: Context<'_, '_, '_, 'info, RolloverBatchStateMerkleTree<'info>>,
        bump: u8,
    ) -> Result<()> {
        let account = BatchedMerkleTreeAccount::state_tree_from_account_info_mut(
            &ctx.accounts.old_state_merkle_tree,
        )
        .map_err(ProgramError::from)?;
        check_forester(
            &account.get_metadata().metadata,
            ctx.accounts.authority.key(),
            ctx.accounts.old_state_merkle_tree.key(),
            &mut ctx.accounts.registered_forester_pda,
            DEFAULT_WORK_V1,
        )?;
        check_cpi_context(
            ctx.accounts.cpi_context_account.to_account_info(),
            &ctx.accounts.protocol_config_pda.config,
        )?;

        process_rollover_batch_state_merkle_tree(&ctx, bump)?;

        process_initialize_cpi_context(
            bump,
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.cpi_context_account.to_account_info(),
            ctx.accounts.new_state_merkle_tree.to_account_info(),
            ctx.accounts.light_system_program.to_account_info(),
        )
    }

    pub fn migrate_state<'info>(
        ctx: Context<'_, '_, '_, 'info, MigrateState<'info>>,
        bump: u8,
        inputs: MigrateLeafParams,
    ) -> Result<()> {
        check_forester(
            &ctx.accounts.merkle_tree.load()?.metadata,
            ctx.accounts.authority.key(),
            ctx.accounts.merkle_tree.key(),
            &mut Some(ctx.accounts.registered_forester_pda.clone()),
            DEFAULT_WORK_V1,
        )?;
        process_migrate_state(&ctx, bump, inputs)
    }
}

/// if registered_forester_pda is not None check forester eligibility and network_fee is not 0
/// if metadata.forester == authority can forest
/// else throw error
pub fn check_forester(
    metadata: &MerkleTreeMetadata,
    authority: Pubkey,
    queue: Pubkey,
    registered_forester_pda: &mut Option<Account<'_, ForesterEpochPda>>,
    num_work_items: u64,
) -> Result<()> {
    if let Some(forester_pda) = registered_forester_pda.as_mut() {
        // Checks forester:
        // - signer
        // - eligibility
        // - increments work counter
        ForesterEpochPda::check_forester_in_program(
            forester_pda,
            &authority,
            &queue,
            num_work_items,
        )?;
        if metadata.rollover_metadata.network_fee == 0 {
            return err!(RegistryError::InvalidNetworkFee);
        }
        Ok(())
    } else if metadata.access_metadata.forester == authority.into() {
        Ok(())
    } else {
        err!(RegistryError::InvalidSigner)
    }
}
