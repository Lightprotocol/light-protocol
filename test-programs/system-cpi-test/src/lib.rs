#![allow(clippy::too_many_arguments)]
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use light_system_program::invoke::processor::CompressedProof;
pub mod create_pda;
pub use create_pda::*;
pub mod sdk;
use light_system_program::NewAddressParamsPacked;
pub mod invalidate_not_owned_account;
pub use invalidate_not_owned_account::*;
use light_system_program::sdk::compressed_account::PackedCompressedAccountWithMerkleContext;
use light_system_program::sdk::CompressedCpiContext;

declare_id!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

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
        msg!("ID {:?}", ID);
        let (pubkey, bump) = Pubkey::find_program_address(&[b"cpi_authority"], &ID);
        msg!("pubkey {:?}", pubkey);
        let accounts = account_compression::cpi::accounts::AppendLeaves {
            authority: ctx.accounts.cpi_signer.to_account_info(),
            fee_payer: ctx.accounts.signer.to_account_info(),
            registered_program_pda: Some(ctx.accounts.registered_program_pda.to_account_info()),
            log_wrapper: ctx.accounts.noop_program.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let bump = &[bump];
        let seeds = [&[b"cpi_authority".as_slice(), bump][..]];
        let mut cpi_context = CpiContext::new_with_signer(
            ctx.accounts.account_compression_program.to_account_info(),
            accounts,
            &seeds,
        );
        cpi_context.remaining_accounts = vec![ctx.accounts.merkle_tree.to_account_info()];

        account_compression::cpi::append_leaves_to_merkle_trees(cpi_context, vec![(0, [1u8; 32])])?;

        Ok(())
    }
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
}
