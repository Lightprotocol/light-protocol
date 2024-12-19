#![allow(deprecated)]
use anchor_lang::{prelude::*, solana_program::account_info::AccountInfo};
use anchor_spl::{token::TokenAccount, token_interface};

use crate::{
    process_transfer::get_cpi_signer_seeds, CompressedTokenInstructionDataTransfer,
    TransferInstruction, POOL_SEED,
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
    program_id: &Pubkey,
    token_pool_pubkey: &Pubkey,
    bump: &[u8],
) -> Result<()> {
    if check_spl_token_pool_derivation(mint_bytes, program_id, token_pool_pubkey, bump) {
        Ok(())
    } else {
        err!(crate::ErrorCode::InvalidTokenPoolPda)
    }
}

fn check_spl_token_pool_derivation(
    mint_bytes: &[u8],
    program_id: &Pubkey,
    token_pool_pubkey: &Pubkey,
    bump: &[u8],
) -> bool {
    let seeds = [POOL_SEED, mint_bytes, bump];
    let seeds = if bump[0] == 0 {
        &seeds[..2]
    } else {
        &seeds[..]
    };
    let (pda, _) = Pubkey::find_program_address(seeds, program_id);
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

    let mut token_pool_bumps = (0..crate::NUM_MAX_POOL_ACCOUNTS).collect::<Vec<u8>>();

    for i in 0..crate::NUM_MAX_POOL_ACCOUNTS {
        if i != 0 {
            token_pool_pda = ctx.remaining_accounts[i as usize - 1].to_account_info();
        }
        let token_pool_amount =
            TokenAccount::try_deserialize(&mut &token_pool_pda.data.borrow()[..])
                .map_err(|_| crate::ErrorCode::InvalidTokenPoolPda)?
                .amount;
        let witdrawal_amount = std::cmp::min(amount, token_pool_amount);

        for (index, i) in token_pool_bumps.iter().enumerate() {
            match check_spl_token_pool_derivation(
                mint_bytes.as_slice(),
                &crate::ID,
                &token_pool_pda.key(),
                &[*i],
            ) {
                true => {
                    transfer(
                        token_pool_pda.to_account_info(),
                        recipient.to_account_info(),
                        ctx.accounts.cpi_authority_pda.to_account_info(),
                        ctx.accounts
                            .token_program
                            .as_ref()
                            .unwrap()
                            .to_account_info(),
                        witdrawal_amount,
                    )?;
                    token_pool_bumps.remove(index);
                    return Ok(());
                }
                false => {}
            }
        }

        amount = amount.saturating_sub(witdrawal_amount);
        msg!("Amount: {}", amount);
        msg!("Witdrawal Amount: {}", witdrawal_amount);
        if amount == 0 {
            return Ok(());
        }
    }
    unreachable!("If this state is reached this means more compressed tokens exist than spl tokens which is a bug.");
}

pub fn compress_spl_tokens<'info>(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
) -> Result<()> {
    let mut recipient_token_pool = match ctx.accounts.token_pool_pda.as_ref() {
        Some(token_pool_pda) => token_pool_pda.to_account_info(),
        None => return err!(crate::ErrorCode::CompressedPdaUndefinedForCompress),
    };
    let amount = match inputs.compress_or_decompress_amount {
        Some(amount) => amount,
        None => return err!(crate::ErrorCode::DeCompressAmountUndefinedForCompress),
    };

    let mint_bytes = inputs.mint.to_bytes();

    for i in 0..crate::NUM_MAX_POOL_ACCOUNTS {
        match check_spl_token_pool_derivation(
            mint_bytes.as_slice(),
            &crate::ID,
            &recipient_token_pool.key(),
            &[i],
        ) {
            true => {
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
            false => {
                recipient_token_pool = ctx.remaining_accounts[i as usize].to_account_info();
                TokenAccount::try_deserialize(&mut &recipient_token_pool.data.borrow()[..])
                    .map_err(|_| crate::ErrorCode::InvalidTokenPoolPda)?;
            }
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
