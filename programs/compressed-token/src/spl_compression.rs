use anchor_lang::{prelude::*, solana_program::account_info::AccountInfo};
use anchor_spl::token::Transfer;

use crate::{
    process_transfer::get_cpi_signer_seeds, CompressedTokenInstructionDataTransfer,
    TransferInstruction,
};

pub fn process_compression_or_decompression<'info>(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
) -> Result<()> {
    if inputs.is_compress {
        compress_spl_tokens(inputs, ctx)
    } else {
        decompress_spl_tokens(inputs, ctx)
    }
}

pub fn decompress_spl_tokens<'info>(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
) -> Result<()> {
    let recipient = match ctx.accounts.compress_or_decompress_token_account.as_ref() {
        Some(compression_recipient) => compression_recipient.to_account_info(),
        None => return err!(crate::ErrorCode::DecompressRecipientUndefinedForDecompress),
    };
    let token_pool_pda = match ctx.accounts.token_pool_pda.as_ref() {
        Some(token_pool_pda) => token_pool_pda.to_account_info(),
        None => return err!(crate::ErrorCode::CompressedPdaUndefinedForDecompress),
    };
    let amount = match inputs.compress_or_decompress_amount {
        Some(amount) => amount,
        None => return err!(crate::ErrorCode::DeCompressAmountUndefinedForDecompress),
    };
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
    let amount = match inputs.compress_or_decompress_amount {
        Some(amount) => amount,
        None => return err!(crate::ErrorCode::DeCompressAmountUndefinedForCompress),
    };

    transfer_compress(
        &ctx.accounts
            .compress_or_decompress_token_account
            .as_ref()
            .unwrap()
            .to_account_info(),
        &recipient,
        &ctx.accounts.authority.to_account_info(),
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
    let signer_seeds = get_cpi_signer_seeds();
    let signer_seeds_ref = &[&signer_seeds[..]];
    let accounts = Transfer {
        from: from.to_account_info(),
        to: to.to_account_info(),
        authority: authority.to_account_info(),
    };
    let cpi_ctx =
        CpiContext::new_with_signer(token_program.to_account_info(), accounts, signer_seeds_ref);
    anchor_spl::token::transfer(cpi_ctx, amount)
}

pub fn transfer_compress<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let accounts = Transfer {
        from: from.to_account_info(),
        to: to.to_account_info(),
        authority: authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(token_program.to_account_info(), accounts);
    anchor_spl::token::transfer(cpi_ctx, amount)
}
