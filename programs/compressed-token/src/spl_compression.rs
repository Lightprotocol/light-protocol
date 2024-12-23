#![allow(deprecated)]
use anchor_lang::{prelude::*, solana_program::account_info::AccountInfo};
use anchor_spl::{token::TokenAccount, token_interface};

use crate::{
    constants::{NUM_MAX_POOL_ACCOUNTS, POOL_SEED},
    process_transfer::get_cpi_signer_seeds,
    CompressedTokenInstructionDataTransfer, TransferInstruction,
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

pub fn spl_token_pool_derivation(
    mint_bytes: &[u8],
    token_pool_pubkey: &Pubkey,
    bump: &[u8],
) -> Result<()> {
    if check_spl_token_pool_derivation(mint_bytes, token_pool_pubkey, bump) {
        Ok(())
    } else {
        err!(crate::ErrorCode::InvalidTokenPoolPda)
    }
}

pub fn check_spl_token_pool_derivation(
    mint_bytes: &[u8],
    token_pool_pubkey: &Pubkey,
    bump: &[u8],
) -> bool {
    let seeds = [POOL_SEED, mint_bytes, bump];
    let seeds = if bump[0] == 0 {
        &seeds[..2]
    } else {
        &seeds[..]
    };
    let (pda, _) = Pubkey::find_program_address(seeds, &crate::ID);
    pda == *token_pool_pubkey
}

pub fn decompress_spl_tokens<'info>(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
) -> Result<()> {
    let recipient = match ctx.accounts.compress_or_decompress_token_account.as_ref() {
        Some(compression_recipient) => compression_recipient.to_account_info(),
        None => return err!(crate::ErrorCode::DecompressRecipientUndefinedForDecompress),
    };
    let mut token_pool_pda = match ctx.accounts.token_pool_pda.as_ref() {
        Some(token_pool_pda) => token_pool_pda.to_account_info(),
        None => return err!(crate::ErrorCode::CompressedPdaUndefinedForDecompress),
    };
    let mut amount = match inputs.compress_or_decompress_amount {
        Some(amount) => amount,
        None => return err!(crate::ErrorCode::DeCompressAmountUndefinedForDecompress),
    };
    let mint_bytes = inputs.mint.to_bytes();

    let mut token_pool_bumps = (0..NUM_MAX_POOL_ACCOUNTS).collect::<Vec<u8>>();

    for i in 0..NUM_MAX_POOL_ACCOUNTS {
        if i != 0 {
            token_pool_pda = ctx.remaining_accounts[i as usize - 1].to_account_info();
        }
        let token_pool_amount =
            TokenAccount::try_deserialize(&mut &token_pool_pda.data.borrow()[..])
                .map_err(|_| crate::ErrorCode::InvalidTokenPoolPda)?
                .amount;
        let withdrawal_amount = std::cmp::min(amount, token_pool_amount);
        if withdrawal_amount == 0 {
            continue;
        }
        let mut remove_index = 0;
        for (index, i) in token_pool_bumps.iter().enumerate() {
            if check_spl_token_pool_derivation(mint_bytes.as_slice(), &token_pool_pda.key(), &[*i])
            {
                transfer(
                    token_pool_pda.to_account_info(),
                    recipient.to_account_info(),
                    ctx.accounts.cpi_authority_pda.to_account_info(),
                    ctx.accounts
                        .token_program
                        .as_ref()
                        .unwrap()
                        .to_account_info(),
                    withdrawal_amount,
                )?;
                remove_index = index;
            }
        }
        token_pool_bumps.remove(remove_index);

        amount = amount.saturating_sub(withdrawal_amount);
        if amount == 0 {
            return Ok(());
        }
    }
    msg!("Remaining amount: {}.", amount);
    msg!("Token pool account balance insufficient for decompression. \nTry to pass more token pool accounts.");
    err!(crate::ErrorCode::FailedToDecompress)
}

pub fn compress_spl_tokens<'info>(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
) -> Result<()> {
    let recipient_token_pool = match ctx.accounts.token_pool_pda.as_ref() {
        Some(token_pool_pda) => token_pool_pda.to_account_info(),
        None => return err!(crate::ErrorCode::CompressedPdaUndefinedForCompress),
    };
    let amount = match inputs.compress_or_decompress_amount {
        Some(amount) => amount,
        None => return err!(crate::ErrorCode::DeCompressAmountUndefinedForCompress),
    };

    let mint_bytes = inputs.mint.to_bytes();

    for i in 0..NUM_MAX_POOL_ACCOUNTS {
        if check_spl_token_pool_derivation(mint_bytes.as_slice(), &recipient_token_pool.key(), &[i])
        {
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
            )?;
            return Ok(());
        }
    }
    err!(crate::ErrorCode::InvalidTokenPoolPda)
}

pub fn transfer<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let signer_seeds = get_cpi_signer_seeds();
    let signer_seeds_ref = &[&signer_seeds[..]];

    let accounts = token_interface::Transfer {
        from,
        to,
        authority,
    };
    let cpi_ctx = CpiContext::new_with_signer(token_program, accounts, signer_seeds_ref);
    anchor_spl::token_interface::transfer(cpi_ctx, amount)
}

pub fn transfer_compress<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let accounts = token_interface::Transfer {
        from,
        to,
        authority,
    };
    let cpi_ctx = CpiContext::new(token_program, accounts);
    anchor_spl::token_interface::transfer(cpi_ctx, amount)
}
