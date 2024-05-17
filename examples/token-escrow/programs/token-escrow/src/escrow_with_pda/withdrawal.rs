use anchor_lang::prelude::*;
use light_compressed_token::{
    CompressedTokenInstructionDataTransfer, InputTokenDataWithContext,
    PackedTokenTransferOutputData,
};
use light_system_program::invoke::processor::CompressedProof;

use crate::{create_change_output_compressed_token_account, EscrowError};

pub fn process_escrow_compressed_tokens_with_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
    lock_up_time: u64,
    escrow_amount: u64,
    proof: CompressedProof,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_state_merkle_tree_account_indices: Vec<u8>,
) -> Result<()> {
    // set timelock
    let current_slot = Clock::get()?.slot;
    ctx.accounts.timelock_pda.slot = current_slot.checked_add(lock_up_time).unwrap();

    let escrow_token_data = PackedTokenTransferOutputData {
        amount: escrow_amount,
        owner: ctx.accounts.token_owner_pda.key(),
        lamports: None,
        merkle_tree_index: output_state_merkle_tree_account_indices[0],
    };
    let change_token_data = create_change_output_compressed_token_account(
        &input_token_data_with_context,
        &[escrow_token_data],
        &ctx.accounts.signer.key(),
        output_state_merkle_tree_account_indices[1],
    );
    let output_compressed_accounts = vec![escrow_token_data, change_token_data];

    cpi_compressed_token_transfer(
        &ctx,
        proof,
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
    )
}

/// Allows the owner to withdraw compressed tokens from the escrow account,
/// provided the lockup time has expired.
pub fn process_withdraw_compressed_escrow_tokens_with_pda<'info>(
    ctx: Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
    bump: u8,
    withdrawal_amount: u64,
    proof: CompressedProof,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_state_merkle_tree_account_indices: Vec<u8>,
) -> Result<()> {
    let current_slot = Clock::get()?.slot;
    if current_slot < ctx.accounts.timelock_pda.slot {
        return err!(EscrowError::EscrowLocked);
    }

    let escrow_token_data = PackedTokenTransferOutputData {
        amount: withdrawal_amount,
        owner: ctx.accounts.signer.key(),
        lamports: None,
        merkle_tree_index: output_state_merkle_tree_account_indices[0],
    };
    let change_token_data = create_change_output_compressed_token_account(
        &input_token_data_with_context,
        &[escrow_token_data],
        &ctx.accounts.token_owner_pda.key(),
        output_state_merkle_tree_account_indices[1],
    );
    let output_compressed_accounts = vec![escrow_token_data, change_token_data];

    withdrawal_cpi_compressed_token_transfer(
        &ctx,
        bump,
        proof,
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
    )
}

#[derive(Accounts)]
pub struct EscrowCompressedTokensWithPda<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    /// CHECK:
    #[account(seeds = [b"escrow".as_slice(), signer.key.to_bytes().as_slice()], bump)]
    pub token_owner_pda: AccountInfo<'info>,
    pub compressed_token_program:
        Program<'info, light_compressed_token::program::LightCompressedToken>,
    pub light_system_program: Program<'info, light_system_program::program::LightSystemProgram>,
    pub account_compression_program:
        Program<'info, account_compression::program::AccountCompression>,
    /// CHECK:
    pub account_compression_authority: AccountInfo<'info>,
    /// CHECK:
    pub compressed_token_cpi_authority_pda: AccountInfo<'info>,
    /// CHECK:
    pub registered_program_pda: AccountInfo<'info>,
    /// CHECK:
    pub noop_program: AccountInfo<'info>,
    #[account(init_if_needed, seeds = [b"timelock".as_slice(), signer.key.to_bytes().as_slice()],bump, payer = signer, space = 8 + 8)]
    pub timelock_pda: Account<'info, EscrowTimeLock>,
    pub system_program: Program<'info, System>,
    #[account(mut)]
    pub cpi_context_account:
        Account<'info, light_system_program::invoke_cpi::account::CpiContextAccount>,
}

#[account]
pub struct EscrowTimeLock {
    pub slot: u64,
}

#[inline(never)]
pub fn cpi_compressed_token_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
    proof: CompressedProof,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<PackedTokenTransferOutputData>,
) -> Result<()> {
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof: Some(proof),
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
        is_compress: false,
        compression_amount: None,
        cpi_context: None,
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let cpi_accounts = light_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        self_program: ctx.accounts.compressed_token_program.to_account_info(),
        cpi_authority_pda: ctx
            .accounts
            .compressed_token_cpi_authority_pda
            .to_account_info(),
        light_system_program: ctx.accounts.light_system_program.to_account_info(),
        token_pool_pda: None,
        decompress_token_account: None,
        token_program: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_context_account: ctx.accounts.cpi_context_account.to_account_info(),
    };

    let mut cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    light_compressed_token::cpi::transfer(cpi_ctx, inputs)?;
    Ok(())
}

#[inline(never)]
pub fn withdrawal_cpi_compressed_token_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
    bump: u8,
    proof: CompressedProof,
    mint: Pubkey,
    signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<PackedTokenTransferOutputData>,
) -> Result<()> {
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof: Some(proof),
        mint,
        signer_is_delegate,
        input_token_data_with_context,
        output_compressed_accounts,
        is_compress: false,
        compression_amount: None,
        cpi_context: None,
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let bump = &[bump];
    let signer_bytes = ctx.accounts.signer.key.to_bytes();
    let seeds = [b"escrow".as_slice(), signer_bytes.as_slice(), bump];

    let signer_seeds = &[&seeds[..]];
    let cpi_accounts = light_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.token_owner_pda.to_account_info(),
        authority: ctx.accounts.token_owner_pda.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        self_program: ctx.accounts.compressed_token_program.to_account_info(),
        cpi_authority_pda: ctx
            .accounts
            .compressed_token_cpi_authority_pda
            .to_account_info(),
        light_system_program: ctx.accounts.light_system_program.to_account_info(),
        token_pool_pda: None,
        decompress_token_account: None,
        token_program: None,
        system_program: ctx.accounts.system_program.to_account_info(),
        cpi_context_account: ctx.accounts.cpi_context_account.to_account_info(),
    };

    let mut cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    cpi_ctx.remaining_accounts = ctx.remaining_accounts.to_vec();
    light_compressed_token::cpi::transfer(cpi_ctx, inputs)?;
    Ok(())
}
