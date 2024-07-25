use anchor_lang::prelude::*;
use light_compressed_token::process_transfer::{
    CompressedTokenInstructionDataTransfer, InputTokenDataWithContext,
    PackedTokenTransferOutputData,
};
use light_system_program::invoke::processor::CompressedProof;

use crate::{
    create_change_output_compressed_token_account, EscrowCompressedTokensWithPda, EscrowError,
};

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
        tlv: None,
    };
    let change_token_data = create_change_output_compressed_token_account(
        &input_token_data_with_context,
        &[escrow_token_data.clone()],
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

#[inline(never)]
pub fn withdrawal_cpi_compressed_token_transfer<'info>(
    ctx: &Context<'_, '_, '_, 'info, EscrowCompressedTokensWithPda<'info>>,
    bump: u8,
    proof: CompressedProof,
    mint: Pubkey,
    _signer_is_delegate: bool,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_compressed_accounts: Vec<PackedTokenTransferOutputData>,
) -> Result<()> {
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof: Some(proof),
        mint,
        delegated_transfer: None,
        input_token_data_with_context,
        output_compressed_accounts,
        is_compress: false,
        compress_or_decompress_amount: None,
        cpi_context: None,
        lamports_change_account_merkle_tree_index: None,
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    let bump = &[bump];
    let signer_bytes = ctx.accounts.signer.key.to_bytes();
    let seeds = [b"escrow".as_slice(), signer_bytes.as_slice(), bump];

    let signer_seeds = &[&seeds[..]];
    let cpi_accounts = light_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
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
        compress_or_decompress_token_account: None,
        token_program: None,
        system_program: ctx.accounts.system_program.to_account_info(),
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
