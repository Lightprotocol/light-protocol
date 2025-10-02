#![allow(clippy::too_many_arguments)]
#![allow(deprecated)]
use account_compression::{program::AccountCompression, utils::constants::CPI_AUTHORITY_PDA_SEED};
use anchor_lang::{prelude::*, solana_program::pubkey::Pubkey};
pub mod create_pda;
pub use create_pda::*;
pub mod cpi_context_event;
pub mod cpi_context_event_inputs;
pub mod invalidate_not_owned_account;
pub mod sdk;
use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};
pub use cpi_context_event::*;
pub use cpi_context_event_inputs::*;
pub use invalidate_not_owned_account::*;
use light_compressed_account::{
    compressed_account::{
        PackedCompressedAccountWithMerkleContext, PackedReadOnlyCompressedAccount,
    },
    instruction_data::{
        compressed_proof::CompressedProof,
        cpi_context::CompressedCpiContext,
        data::{NewAddressParamsPacked, PackedReadOnlyAddress},
    },
};
use light_sdk::derive_light_cpi_signer;
use light_sdk_types::CpiSigner;

declare_id!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

pub const LIGHT_CPI_SIGNER: CpiSigner =
    derive_light_cpi_signer!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

#[program]
pub mod system_cpi_test {

    use light_compressed_account::instruction_data::insert_into_queues::InsertIntoQueuesInstructionDataMut;

    use super::*;

    pub fn create_compressed_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, CreateCompressedPda<'info>>,
        data: [u8; 31],
        proof: Option<CompressedProof>,
        new_address_parameters: NewAddressParamsPacked,
        owner_program: Pubkey,
        signer_is_program: CreatePdaMode,
        bump: u8,
        cpi_context: Option<CompressedCpiContext>,
        read_only_address: Option<Vec<PackedReadOnlyAddress>>,
        read_only_accounts: Option<Vec<PackedReadOnlyCompressedAccount>>,
        input_accounts: Option<Vec<PackedCompressedAccountWithMerkleContext>>,
    ) -> Result<()> {
        process_create_pda(
            ctx,
            data,
            proof,
            new_address_parameters,
            owner_program,
            cpi_context,
            signer_is_program,
            bump,
            read_only_address,
            read_only_accounts,
            input_accounts,
        )
    }

    pub fn with_input_accounts<'info>(
        ctx: Context<'_, '_, '_, 'info, InvalidateNotOwnedCompressedAccount<'info>>,
        compressed_account: PackedCompressedAccountWithMerkleContext,
        proof: Option<CompressedProof>,
        bump: u8,
        mode: WithInputAccountsMode,
        cpi_context: Option<CompressedCpiContext>,
        token_transfer_data: Option<TokenTransferData>,
    ) -> Result<()> {
        process_with_input_accounts(
            ctx,
            compressed_account,
            proof,
            bump,
            mode,
            cpi_context,
            token_transfer_data,
        )
    }

    pub fn insert_into_queues<'info>(
        ctx: Context<'_, '_, '_, 'info, InsertIntoQueues<'info>>,
        is_batched: bool,
        cpi_bump: u8,
    ) -> Result<()> {
        let (_, bump) = Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &ID);
        let accounts = account_compression::cpi::accounts::GenericInstruction {
            authority: ctx.accounts.cpi_signer.to_account_info(),
        };
        let bump = &[bump];
        let seeds = [&[CPI_AUTHORITY_PDA_SEED, bump][..]];
        let mut cpi_context = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            &seeds,
        );
        cpi_context.remaining_accounts = vec![
            ctx.accounts.registered_program_pda.to_account_info(),
            ctx.accounts.state_merkle_tree.to_account_info(),
            ctx.accounts.nullifier_queue.to_account_info(),
            ctx.accounts.address_tree.to_account_info(),
            ctx.accounts.address_queue.to_account_info(),
        ];

        let mut bytes =
            vec![
                0u8;
                InsertIntoQueuesInstructionDataMut::required_size_for_capacity(1, 1, 1, 1, 1, 1)
            ];
        let (mut inputs, _) =
            InsertIntoQueuesInstructionDataMut::new_at(&mut bytes, 1, 1, 1, 1, 1, 1).unwrap();
        inputs.num_queues = 1;
        inputs.num_output_queues = 1;
        inputs.num_address_queues = 1;
        inputs.leaves[0].leaf = [1u8; 32];
        inputs.leaves[0].account_index = if is_batched { 1 } else { 0 };
        inputs.nullifiers[0].account_hash = [2u8; 32];
        inputs.nullifiers[0].leaf_index = 1.into();
        inputs.nullifiers[0].prove_by_index = 0;
        inputs.nullifiers[0].queue_index = 1;
        inputs.nullifiers[0].tree_index = 0;

        inputs.addresses[0].address = [3u8; 32];
        inputs.addresses[0].queue_index = 3;
        inputs.addresses[0].tree_index = 2;
        inputs.set_invoked_by_program(true);
        inputs.bump = cpi_bump;
        account_compression::cpi::insert_into_queues(cpi_context, bytes)?;

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
            None,
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
        additional_bytes: u64,
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
            None,
            merkle_tree_config,
            queue_config,
            additional_bytes,
        )
    }

    pub fn cpi_context_indexing<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, GenericAnchorAccounts<'info>>,
        mode: u8,
    ) -> Result<()> {
        process_cpi_context_indexing(ctx, mode)
    }

    pub fn cpi_context_indexing_inputs<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, GenericAnchorAccounts<'info>>,
        mode: u8,
        leaf_indices: [u8; 3],
    ) -> Result<()> {
        process_cpi_context_indexing_inputs(ctx, mode, leaf_indices)
    }
}

#[derive(Accounts)]
pub struct InitializeAddressMerkleTreeAndQueue<'info> {
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
pub struct InsertIntoQueues<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub account_compression_program: Program<'info, AccountCompression>,
    /// CHECK:
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK:
    pub noop_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    pub cpi_signer: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub state_merkle_tree: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub nullifier_queue: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub address_tree: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub address_queue: AccountInfo<'info>,
}
