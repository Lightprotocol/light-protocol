#![allow(deprecated)]
use anchor_lang::{prelude::*, solana_program::account_info::AccountInfo};
use anchor_spl::{token::TokenAccount, token_interface};

use crate::{
    check_spl_token_pool_derivation,
    constants::{NUM_MAX_POOL_ACCOUNTS, POOL_SEED},
    process_transfer::{get_cpi_signer_seeds, CompressedTokenInstructionDataTransfer},
    ErrorCode, TransferInstruction,
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

pub fn check_spl_token_pool_derivation_with_index(
    mint_bytes: &[u8],
    token_pool_pubkey: &Pubkey,
    pool_index: &[u8],
) -> Result<()> {
    if is_valid_token_pool_pda(mint_bytes, token_pool_pubkey, pool_index, None)? {
        Ok(())
    } else {
        err!(ErrorCode::InvalidTokenPoolPda)
    }
}

#[inline(always)]
pub fn is_valid_token_pool_pda(
    mint_bytes: &[u8],
    token_pool_pubkey: &Pubkey,
    pool_index: &[u8],
    bump: Option<u8>,
) -> Result<bool> {
    let pool_index = if pool_index[0] == 0 { &[] } else { pool_index };
    let pda = if let Some(bump) = bump {
        #[cfg(target_os = "solana")]
        {
            let seeds = [POOL_SEED, mint_bytes, pool_index];
            pinocchio_pubkey::derive_address(&seeds, Some(bump), &crate::ID.to_bytes()).into()
        }
        #[cfg(not(target_os = "solana"))]
        {
            let seeds = [POOL_SEED, mint_bytes, pool_index, &[bump]];
            Pubkey::create_program_address(&seeds[..], &crate::ID).map_err(ProgramError::from)?
        }
    } else {
        let seeds = [POOL_SEED, mint_bytes, pool_index];
        Pubkey::find_program_address(&seeds[..], &crate::ID).0
    };
    Ok(pda == *token_pool_pubkey)
}

pub fn decompress_spl_tokens<'info>(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
) -> Result<()> {
    let recipient = match ctx.accounts.compress_or_decompress_token_account.as_ref() {
        Some(compression_recipient) => compression_recipient.to_account_info(),
        None => return err!(ErrorCode::DecompressRecipientUndefinedForDecompress),
    };
    let token_pool_pda = match ctx.accounts.token_pool_pda.as_ref() {
        Some(token_pool_pda) => token_pool_pda.to_account_info(),
        None => return err!(ErrorCode::CompressedPdaUndefinedForDecompress),
    };
    let amount = match inputs.compress_or_decompress_amount {
        Some(amount) => amount,
        None => return err!(ErrorCode::DeCompressAmountUndefinedForDecompress),
    };
    invoke_token_program_with_multiple_token_pool_accounts::<false>(
        ctx.remaining_accounts,
        &inputs.mint.key().to_bytes(),
        None,
        Some(recipient),
        ctx.accounts.cpi_authority_pda.to_account_info(),
        ctx.accounts
            .token_program
            .as_ref()
            .unwrap()
            .to_account_info(),
        token_pool_pda,
        amount,
    )
}

/// Executes a token program instruction with multiple token pool accounts.
/// Supported instructions are burn and transfer to decompress spl tokens.
/// Logic:
/// 1. Iterate over at most NUM_MAX_POOL_ACCOUNTS token pool accounts.
/// 2. Start with passed in token pool account.
/// 3. Determine whether complete amount can be transferred or burned.
/// 4. Skip if action amount is zero.
/// 5. Check if the token pool account is derived from the mint.
/// 6. Return error if the token pool account is not derived
///    from any combination of mint and bump.
/// 7. Burn or transfer the amount from the token pool account.
/// 8. Remove bump from the list of bumps.
/// 9. Reduce the amount by the transferred or burned amount.
/// 10. Continue until the amount is zero.
/// 11. Return if complete amount has been transferred or burned.
/// 12. Return error if the amount is not zero and the number of accounts has been exhausted.
#[allow(clippy::too_many_arguments)]
pub fn invoke_token_program_with_multiple_token_pool_accounts<'info, const IS_BURN: bool>(
    remaining_accounts: &[AccountInfo<'info>],
    mint_bytes: &[u8; 32],
    mint: Option<AccountInfo<'info>>,
    recipient: Option<AccountInfo<'info>>,
    cpi_authority_pda: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    mut token_pool_pda: AccountInfo<'info>,
    mut amount: u64,
) -> Result<()> {
    let mut token_pool_indices: Vec<u8> = (0..NUM_MAX_POOL_ACCOUNTS).collect();
    // 1. iterate over at most NUM_MAX_POOL_ACCOUNTS token pool accounts.
    for i in 0..NUM_MAX_POOL_ACCOUNTS {
        // 2. Start with passed in token pool account.token_pool_indices
        if i != 0 {
            token_pool_pda = remaining_accounts[i as usize - 1].to_account_info();
        }
        let token_pool_amount =
            TokenAccount::try_deserialize(&mut &token_pool_pda.data.borrow()[..])
                .map_err(|_| ErrorCode::InvalidTokenPoolPda)?
                .amount;
        // 3. Determine whether complete amount can be transferred or burned.
        let action_amount = std::cmp::min(amount, token_pool_amount);
        // 4. Skip if action amount is zero.
        if action_amount == 0 {
            continue;
        }
        // 5. Check if the token pool account is derived from the mint for any bump.
        for (index, i) in token_pool_indices.iter().enumerate() {
            if is_valid_token_pool_pda(mint_bytes.as_slice(), &token_pool_pda.key(), &[*i], None)? {
                // 7. Burn or transfer the amount from the token pool account.
                if IS_BURN {
                    crate::burn::spl_burn_cpi(
                        mint.clone().unwrap(),
                        cpi_authority_pda.to_account_info(),
                        token_pool_pda.to_account_info(),
                        token_program.to_account_info(),
                        action_amount,
                        token_pool_amount,
                    )?;
                } else {
                    crate::spl_compression::spl_token_transfer_cpi_with_signer(
                        token_pool_pda.to_account_info(),
                        recipient.clone().unwrap(),
                        cpi_authority_pda.to_account_info(),
                        token_program.to_account_info(),
                        action_amount,
                    )?;
                }
                // 8. Remove bump from the list of bumps.
                token_pool_indices.remove(index);
                // 9. Reduce the amount by the transferred or burned amount.
                amount = amount.saturating_sub(action_amount);
                break;
            } else if index == token_pool_indices.len() - 1 {
                // 6. Return error if the token pool account is not derived
                //      from any combination of mint and bump.
                return err!(crate::ErrorCode::NoMatchingBumpFound);
            }
        }

        // 10. Continue until the amount is zero.
        // 11. Return if complete amount has been transferred or burned.
        if amount == 0 {
            return Ok(());
        }
    }

    // 12. return error if the amount is not zero and the number of accounts has been exhausted.
    msg!("Remaining amount: {}.", amount);
    if IS_BURN {
        msg!("Token pool account balances insufficient for burn. \nTry to pass more token pool accounts.");
        err!(ErrorCode::FailedToBurnSplTokensFromTokenPool)
    } else {
        msg!("Token pool account balances insufficient for decompression. \nTry to pass more token pool accounts.");
        err!(ErrorCode::FailedToDecompress)
    }
}

pub fn compress_spl_tokens<'info>(
    inputs: &CompressedTokenInstructionDataTransfer,
    ctx: &Context<'_, '_, '_, 'info, TransferInstruction<'info>>,
) -> Result<()> {
    let recipient_token_pool = match ctx.accounts.token_pool_pda.as_ref() {
        Some(token_pool_pda) => token_pool_pda.to_account_info(),
        None => return err!(ErrorCode::CompressedPdaUndefinedForCompress),
    };
    let amount = match inputs.compress_or_decompress_amount {
        Some(amount) => amount,
        None => return err!(ErrorCode::DeCompressAmountUndefinedForCompress),
    };

    check_spl_token_pool_derivation(&recipient_token_pool.key(), &inputs.mint)?;
    spl_token_transfer(
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
    )
}

/// Invoke the spl token burn instruction with cpi authority pda as signer.
/// Used to decompress spl tokens.
pub fn spl_token_transfer_cpi_with_signer<'info>(
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

/// Invoke the spl token transfer instruction with transaction signer.
/// Used to compress spl tokens.
pub fn spl_token_transfer<'info>(
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    let instruction = match *token_program.key {
        spl_token_2022::ID => spl_token_2022::instruction::transfer(
            token_program.key,
            from.key,
            to.key,
            authority.key,
            &[],
            amount,
        ),
        spl_token::ID => spl_token::instruction::transfer(
            token_program.key,
            from.key,
            to.key,
            authority.key,
            &[],
            amount,
        ),
        _ => return Err(anchor_lang::error::ErrorCode::InvalidProgramId.into()),
    }?;

    anchor_lang::solana_program::program::invoke(
        &instruction,
        &[from, to, authority, token_program],
    )?;
    Ok(())
}
