use anchor_lang::{prelude::*, solana_program::account_info::AccountInfo};
use anchor_spl::token::Transfer;

use crate::{CompressedTokenInstructionDataTransfer, TransferInstruction};

pub fn process_compression<'info>(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
) -> Result<()> {
    if inputs.is_compress {
        compress_spl_tokens(inputs, ctx)
    } else if inputs.compression_amount.is_some() {
        decompress_spl_tokens(inputs, ctx)
    } else {
        Ok(())
    }
}

pub fn decompress_spl_tokens<'info>(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
) -> Result<()> {
    let recipient = match ctx.accounts.decompress_token_account.as_ref() {
        Some(compression_recipient) => compression_recipient.to_account_info(),
        None => return err!(crate::ErrorCode::DecompressRecipientUndefinedForDecompress),
    };
    let token_pool_pda = match ctx.accounts.token_pool_pda.as_ref() {
        Some(token_pool_pda) => token_pool_pda.to_account_info(),
        None => return err!(crate::ErrorCode::CompressedPdaUndefinedForDecompress),
    };
    let amount = match inputs.compression_amount {
        Some(amount) => amount,
        None => return err!(crate::ErrorCode::DeCompressAmountUndefinedForDecompress),
    };
    msg!("transfer_amount {}", amount);
    msg!("token_pool_pda {:?}", ctx.accounts.token_pool_pda);
    msg!("recipient {:?}", ctx.accounts.decompress_token_account);
    transfer(
        &token_pool_pda,
        &recipient,
        &ctx.accounts.cpi_authority_pda.to_account_info(),
        &ctx.accounts
            .token_program
            .as_ref()
            .unwrap()
            .to_account_info(),
        amount,
    )
}

pub fn compress_spl_tokens<'info>(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
) -> Result<()> {
    let recipient = match ctx.accounts.token_pool_pda.as_ref() {
        Some(token_pool_pda) => token_pool_pda.to_account_info(),
        None => return err!(crate::ErrorCode::CompressedPdaUndefinedForCompress),
    };
    let amount = match inputs.compression_amount {
        Some(amount) => amount,
        None => return err!(crate::ErrorCode::DeCompressAmountUndefinedForCompress),
    };

    transfer(
        &ctx.accounts
            .decompress_token_account
            .as_ref()
            .unwrap()
            .to_account_info(),
        &recipient,
        &ctx.accounts.cpi_authority_pda.to_account_info(),
        &ctx.accounts
            .token_program
            .as_ref()
            .unwrap()
            .to_account_info(),
        amount,
    )
}

pub fn transfer<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let (_, bump) =
        anchor_lang::prelude::Pubkey::find_program_address(&[b"cpi_authority"], &crate::ID);
    let bump = &[bump];
    let seeds = &[&[b"cpi_authority".as_slice(), bump][..]];
    let accounts = Transfer {
        from: from.to_account_info(),
        to: to.to_account_info(),
        authority: authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(token_program.to_account_info(), accounts, seeds);
    anchor_spl::token::transfer(cpi_ctx, amount)
}
