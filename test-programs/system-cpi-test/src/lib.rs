#![allow(clippy::too_many_arguments)]
use account_compression::program::AccountCompression;
use account_compression::utils::constants::CPI_AUTHORITY_PDA_SEED;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_system_program::invoke::processor::CompressedProof;
pub mod create_pda;
pub use create_pda::*;
pub mod sdk;
use light_system_program::NewAddressParamsPacked;
pub mod invalidate_not_owned_account;
use account_compression::{
    AddressMerkleTreeConfig, AddressQueueConfig, NullifierQueueConfig, StateMerkleTreeConfig,
};
pub use invalidate_not_owned_account::*;
use light_system_program::sdk::compressed_account::PackedCompressedAccountWithMerkleContext;
use light_system_program::sdk::CompressedCpiContext;
declare_id!("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy");

#[program]

pub mod system_cpi_test {

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

    pub fn append_leaves_account_compression_program<'info>(
        ctx: Context<'_, '_, '_, 'info, AppendLeavesAccountCompressionProgram<'info>>,
    ) -> Result<()> {
        let (_, bump) = Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &ID);
        let accounts = account_compression::cpi::accounts::AppendLeaves {
            authority: ctx.accounts.cpi_signer.to_account_info(),
            fee_payer: ctx.accounts.signer.to_account_info(),
            registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let bump = &[bump];
        let seeds = [&[CPI_AUTHORITY_PDA_SEED, bump][..]];
        let mut cpi_context = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            &seeds,
        );
        cpi_context.remaining_accounts = vec![ctx.accounts.merkle_tree.to_account_info()];

        account_compression::cpi::append_leaves_to_merkle_trees(cpi_context, vec![(0, [1u8; 32])])?;

        Ok(())
    }

    pub fn insert_into_address_queue<'info>(
        ctx: Context<'_, '_, '_, 'info, AppendLeavesAccountCompressionProgram<'info>>,
    ) -> Result<()> {
        let (_, bump) = Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &ID);
        let accounts = account_compression::cpi::accounts::InsertIntoQueues {
            authority: ctx.accounts.cpi_signer.to_account_info(),
            fee_payer: ctx.accounts.signer.to_account_info(),
            registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let bump = &[bump];
        let seeds = [&[CPI_AUTHORITY_PDA_SEED, bump][..]];
        let mut cpi_context = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            &seeds,
        );
        cpi_context.remaining_accounts = vec![
            ctx.accounts.queue.to_account_info(),
            ctx.accounts.merkle_tree.to_account_info(),
        ];

        account_compression::cpi::insert_addresses(cpi_context, vec![[1u8; 32]])?;

        Ok(())
    }

    pub fn insert_into_nullifier_queue<'info>(
        ctx: Context<'_, '_, '_, 'info, AppendLeavesAccountCompressionProgram<'info>>,
    ) -> Result<()> {
        let (_, bump) = Pubkey::find_program_address(&[CPI_AUTHORITY_PDA_SEED], &ID);
        let accounts = account_compression::cpi::accounts::InsertIntoQueues {
            authority: ctx.accounts.cpi_signer.to_account_info(),
            fee_payer: ctx.accounts.signer.to_account_info(),
            registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let bump = &[bump];
        let seeds = [&[CPI_AUTHORITY_PDA_SEED, bump][..]];
        let mut cpi_context = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            &seeds,
        );
        cpi_context.remaining_accounts = vec![
            ctx.accounts.queue.to_account_info(),
            ctx.accounts.merkle_tree.to_account_info(),
        ];

        account_compression::cpi::insert_into_nullifier_queues(cpi_context, vec![[1u8; 32]])?;

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
pub struct AppendLeavesAccountCompressionProgram<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    /// CHECK:
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK:
    pub noop_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    /// CHECK:
    pub cpi_signer: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    /// CHECK:
    #[account(mut)]
    pub queue: AccountInfo<'info>,
}
