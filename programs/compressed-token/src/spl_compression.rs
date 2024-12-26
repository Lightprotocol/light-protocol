#![allow(deprecated)]
use anchor_lang::{prelude::*, solana_program::account_info::AccountInfo};
use anchor_spl::{token::TokenAccount, token_interface};

use crate::{
    constants::{NUM_MAX_POOL_ACCOUNTS, POOL_SEED},
    is_valid_token_pool_pda,
    process_transfer::get_cpi_signer_seeds,
    CompressedTokenInstructionDataTransfer, ErrorCode, TransferInstruction,
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
        err!(ErrorCode::InvalidTokenPoolPda)
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
///     from any combination of mint and bump.
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
    let mut token_pool_bumps: Vec<u8> = (0..NUM_MAX_POOL_ACCOUNTS).collect();
    // 1. iterate over at most NUM_MAX_POOL_ACCOUNTS token pool accounts.
    for i in 0..NUM_MAX_POOL_ACCOUNTS {
        // 2. Start with passed in token pool account.token_pool_bumps
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
        for (index, i) in token_pool_bumps.iter().enumerate() {
            if check_spl_token_pool_derivation(mint_bytes.as_slice(), &token_pool_pda.key(), &[*i])
            {
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
                token_pool_bumps.remove(index);
                // 9. Reduce the amount by the transferred or burned amount.
                amount = amount.saturating_sub(action_amount);
                break;
            } else if index == token_pool_bumps.len() - 1 {
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

    is_valid_token_pool_pda(&recipient_token_pool.key(), &inputs.mint)?;
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
    let accounts = token_interface::Transfer {
        from,
        to,
        authority,
    };
    let cpi_ctx = CpiContext::new(token_program, accounts);
    anchor_spl::token_interface::transfer(cpi_ctx, amount)
}
