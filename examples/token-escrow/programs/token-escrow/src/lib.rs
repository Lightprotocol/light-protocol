#![allow(clippy::too_many_arguments)]
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;
use psp_compressed_pda::utils::CompressedProof;
use psp_compressed_token::TokenTransferOutputData;
use psp_compressed_token::{CompressedTokenInstructionDataTransfer, InputTokenDataWithContext};
pub mod sdk;

#[error_code]
pub enum EscrowError {
    #[msg("Escrow is locked")]
    EscrowLocked,
}

declare_id!("GRLu2hKaAiMbxpkAM1HeXzks9YeGuz18SEgXEizVvPqX");

#[program]
pub mod token_escrow {

    use super::*;

    /// Escrows compressed tokens, for a certain number of slots.
    /// Transfers compressed tokens to compressed token account owned by cpi_signer.
    /// Tokens are locked for lock_up_time slots.
    pub fn escrow_compressed_tokens_with_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
        lock_up_time: u64,
        escrow_amount: u64,
        proof: Option<CompressedProof>,
        root_indices: Vec<u16>,
        mint: Pubkey,
        signer_is_delegate: bool,
        input_token_data_with_context: Vec<InputTokenDataWithContext>,
        output_state_merkle_tree_account_indices: Vec<u8>,
        pubkey_array: Vec<Pubkey>,
    ) -> Result<()> {
        // set timelock
        let current_slot = Clock::get()?.slot;
        ctx.accounts.timelock_pda.slot = current_slot.checked_add(lock_up_time).unwrap();

        let escrow_token_data = TokenTransferOutputData {
            amount: escrow_amount,
            owner: ctx.accounts.cpi_signer.key(),
            lamports: None,
        };
        let change_token_data = create_change_output_compressed_token_account(
            &input_token_data_with_context,
            &[escrow_token_data],
            &ctx.accounts.signer.key(),
        );
        let output_compressed_accounts = vec![escrow_token_data, change_token_data];

        cpi_compressed_token_transfer(
            &ctx,
            proof,
            root_indices,
            mint,
            signer_is_delegate,
            input_token_data_with_context,
            output_compressed_accounts,
            output_state_merkle_tree_account_indices,
            pubkey_array,
        )
    }

    /// Allows the owner to withdraw compressed tokens from the escrow account,
    /// provided the lockup time has expired.
    pub fn withdraw_compressed_escrow_tokens_with_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
        bump: u8,
        withdrawal_amount: u64,
        proof: Option<CompressedProof>,
        root_indices: Vec<u16>,
        mint: Pubkey,
        signer_is_delegate: bool,
        input_token_data_with_context: Vec<InputTokenDataWithContext>,
        output_state_merkle_tree_account_indices: Vec<u8>,
        pubkey_array: Vec<Pubkey>,
    ) -> Result<()> {
        let current_slot = Clock::get()?.slot;
        if current_slot > ctx.accounts.timelock_pda.slot {
            return err!(EscrowError::EscrowLocked);
        }

        let escrow_token_data = TokenTransferOutputData {
            amount: withdrawal_amount,
            owner: ctx.accounts.signer.key(),
            lamports: None,
        };
        let change_token_data = create_change_output_compressed_token_account(
            &input_token_data_with_context,
            &[escrow_token_data],
            &ctx.accounts.cpi_signer.key(),
        );
        let output_compressed_accounts = vec![escrow_token_data, change_token_data];

        withdrawal_cpi_compressed_token_transfer(
            &ctx,
            bump,
            proof,
            root_indices,
            mint,
            signer_is_delegate,
            input_token_data_with_context,
            output_compressed_accounts,
            output_state_merkle_tree_account_indices,
            pubkey_array,
        )
    }
}

#[derive(Accounts)]
pub struct EscrowCompressedTokensWithPda<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(seeds = [b"escrow".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub cpi_signer: AccountInfo<'info>,
    pub compressed_token_program: Program<'info, psp_compressed_token::program::PspCompressedToken>,
    pub compressed_pda_program: Program<'info, psp_compressed_pda::program::PspCompressedPda>,
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    pub account_compression_authority: AccountInfo<'info>,
    pub compressed_token_cpi_authority_pda: AccountInfo<'info>,
    pub registered_program_pda: AccountInfo<'info>,
    pub noop_program: AccountInfo<'info>,
    #[account(init_if_needed, seeds = [b"timelock".as_slice(), signer.key.to_bytes().as_slice()],bump, payer = signer, space = 8 + 8)]
    pub timelock_pda: Account<'info, EscrowTimeLock>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct EscrowTimeLock {
    pub slot: u64,
}

// TODO: add to light_sdk
/// A helper function that creates a new compressed account with the change output.
/// Input sum - Output sum = Change amount
/// Outputs compressed account with the change amount, and owner of the compressed input accounts.
fn create_change_output_compressed_token_account(
    input_token_data_with_context: &[InputTokenDataWithContext],
    output_compressed_accounts: &[TokenTransferOutputData],
    owner: &Pubkey,
) -> TokenTransferOutputData {
    let input_sum = input_token_data_with_context
        .iter()
        .map(|account| account.amount)
        .sum::<u64>();
    let output_sum = output_compressed_accounts
        .iter()
        .map(|account| account.amount)
        .sum::<u64>();
    let change_amount = input_sum - output_sum;
    TokenTransferOutputData {
        amount: change_amount,
        owner: *owner,
        lamports: None,
    }
}

#[inline(never)]
pub fn cpi_compressed_token_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
    proof: Option<CompressedProof>,
    root_indices: Vec<u16>,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<TokenTransferOutputData>,
    output_state_merkle_tree_account_indices: Vec<u8>,
    pubkey_array: Vec<Pubkey>,
) -> Result<()> {
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof,
        root_indices,
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
        output_state_merkle_tree_account_indices,
        pubkey_array,
        is_compress: false,
        compression_amount: None,
    };

    let inputs = input_struct.try_to_vec()?;

    let cpi_accounts = psp_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        psp_account_compression_authority: ctx
            .accounts
            .account_compression_authority
            .to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        self_program: ctx.accounts.compressed_token_program.to_account_info(),
        cpi_authority_pda: ctx
            .accounts
            .compressed_token_cpi_authority_pda
            .to_account_info(),
        compressed_pda_program: ctx.accounts.compressed_pda_program.to_account_info(),
        token_pool_pda: None,
        decompress_token_account: None,
        token_program: None,
    };

    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    psp_compressed_token::cpi::transfer(cpi_ctx, inputs)?;
    Ok(())
}

#[inline(never)]
pub fn withdrawal_cpi_compressed_token_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
    bump: u8,
    proof: Option<CompressedProof>,
    root_indices: Vec<u16>,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<TokenTransferOutputData>,
    output_state_merkle_tree_account_indices: Vec<u8>,
    pubkey_array: Vec<Pubkey>,
) -> Result<()> {
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof,
        root_indices,
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
        output_state_merkle_tree_account_indices,
        pubkey_array,
        is_compress: false,
        compression_amount: None,
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let bump = &[bump];
    let signer_bytes = ctx.accounts.signer.key.to_bytes();
    let seeds = [b"escrow".as_slice(), signer_bytes.as_slice(), bump];

    let signer_seeds = &[&seeds[..]];
    let cpi_accounts = psp_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.cpi_signer.to_account_info(),
        authority: ctx.accounts.cpi_signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        psp_account_compression_authority: ctx
            .accounts
            .account_compression_authority
            .to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        self_program: ctx.accounts.compressed_token_program.to_account_info(),
        cpi_authority_pda: ctx
            .accounts
            .compressed_token_cpi_authority_pda
            .to_account_info(),
        compressed_pda_program: ctx.accounts.compressed_pda_program.to_account_info(),
        token_pool_pda: None,
        decompress_token_account: None,
        token_program: None,
    };

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    psp_compressed_token::cpi::transfer(cpi_ctx, inputs)?;
    Ok(())
}
