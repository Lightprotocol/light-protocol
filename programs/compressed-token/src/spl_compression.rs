#![allow(deprecated)]
use anchor_lang::{prelude::*, solana_program::account_info::AccountInfo};
use anchor_spl::token::Transfer;

use crate::{
    process_transfer::get_cpi_signer_seeds, CompressedTokenInstructionDataTransfer,
    TransferInstruction, POOL_SEED,
};

pub fn process_compression_or_decompression(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, '_, TransferInstruction>,
) -> Result<()> {
    if inputs.is_compress {
        compress_spl_tokens(inputs, ctx)
    } else {
        decompress_spl_tokens(inputs, ctx)
    }
}

pub fn spl_token_pool_derivation(
    mint: &Pubkey,
    program_id: &Pubkey,
    token_pool_pubkey: &Pubkey,
) -> Result<()> {
    let seeds = &[POOL_SEED, &mint.to_bytes()[..]];
    let (pda, _bump_seed) = Pubkey::find_program_address(seeds, program_id);
    if pda == *token_pool_pubkey {
        Ok(())
    } else {
        err!(crate::ErrorCode::InvalidTokenPoolPda)
    }
}

pub fn decompress_spl_tokens(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, '_, TransferInstruction>,
) -> Result<()> {
    let recipient = match ctx.accounts.compress_or_decompress_token_account.as_ref() {
        Some(compression_recipient) => compression_recipient.to_account_info(),
        None => return err!(crate::ErrorCode::DecompressRecipientUndefinedForDecompress),
    };
    let token_pool_pda = match ctx.accounts.token_pool_pda.as_ref() {
        Some(token_pool_pda) => token_pool_pda.to_account_info(),
        None => return err!(crate::ErrorCode::CompressedPdaUndefinedForDecompress),
    };
    spl_token_pool_derivation(&inputs.mint, &crate::ID, &token_pool_pda.key())?;

    let amount = match inputs.compress_or_decompress_amount {
        Some(amount) => amount,
        None => return err!(crate::ErrorCode::DeCompressAmountUndefinedForDecompress),
    };
    let is_token_22 = match ctx.accounts.token_program.as_ref().unwrap().key() {
        spl_token::ID => Ok(false),
        anchor_spl::token_2022::ID => Ok(true),
        _ => err!(crate::ErrorCode::InvalidTokenProgram),
    }?;
    transfer(
        token_pool_pda,
        recipient,
        ctx.accounts.cpi_authority_pda.to_account_info(),
        ctx.accounts
            .token_program
            .as_ref()
            .unwrap()
            .to_account_info(),
        amount,
        is_token_22,
    )
}

pub fn compress_spl_tokens(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, '_, TransferInstruction>,
) -> Result<()> {
    let recipient_token_pool = match ctx.accounts.token_pool_pda.as_ref() {
        Some(token_pool_pda) => token_pool_pda,
        None => return err!(crate::ErrorCode::CompressedPdaUndefinedForCompress),
    };
    spl_token_pool_derivation(&inputs.mint, &crate::ID, &recipient_token_pool.key())?;
    let amount = match inputs.compress_or_decompress_amount {
        Some(amount) => amount,
        None => return err!(crate::ErrorCode::DeCompressAmountUndefinedForCompress),
    };

    let is_token_22 = match ctx.accounts.token_program.as_ref().unwrap().key() {
        spl_token::ID => Ok(false),
        anchor_spl::token_2022::ID => Ok(true),
        _ => err!(crate::ErrorCode::InvalidTokenProgram),
    }?;
    transfer_compress(
        ctx.accounts
            .compress_or_decompress_token_account
            .as_ref()
            .unwrap()
            .to_account_info(),
        recipient_token_pool.to_account_info(),
        ctx.accounts.authority.to_account_info(),
        ctx.accounts
            .token_program
            .as_ref()
            .unwrap()
            .to_account_info(),
        amount,
        is_token_22,
    )
}

pub fn transfer<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
    token_22: bool,
) -> Result<()> {
    let signer_seeds = get_cpi_signer_seeds();
    let signer_seeds_ref = &[&signer_seeds[..]];

    if token_22 {
        let accounts = anchor_spl::token_2022::Transfer {
            from,
            to,
            authority,
        };
        let cpi_ctx = CpiContext::new_with_signer(
            token_program.to_account_info(),
            accounts,
            signer_seeds_ref,
        );
        anchor_spl::token_2022::transfer(cpi_ctx, amount)
    } else {
        let accounts = Transfer {
            from,
            to,
            authority,
        };
        let cpi_ctx = CpiContext::new_with_signer(token_program, accounts, signer_seeds_ref);
        anchor_spl::token::transfer(cpi_ctx, amount)
    }
}

pub fn transfer_compress<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
    token_22: bool,
) -> Result<()> {
    if token_22 {
        let accounts = anchor_spl::token_2022::Transfer {
            from,
            to,
            authority,
        };
        let cpi_ctx = CpiContext::new(token_program, accounts);
        anchor_spl::token_2022::transfer(cpi_ctx, amount)
    } else {
        let accounts = Transfer {
            from,
            to,
            authority,
        };
        let cpi_ctx = CpiContext::new(token_program, accounts);
        anchor_spl::token::transfer(cpi_ctx, amount)
    }
}
