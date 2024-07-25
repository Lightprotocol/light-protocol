use account_compression::utils::transfer_lamports::transfer_lamports_cpi;
use aligned_sized::*;
use anchor_lang::{
    prelude::*,
    solana_program::{account_info::AccountInfo, pubkey::Pubkey},
    Bumps,
};

use crate::{
    errors::SystemProgramError,
    sdk::accounts::{InvokeAccounts, SignerAccounts},
    InstructionDataInvoke,
};

#[account]
#[aligned_sized(anchor)]
pub struct CompressedSolPda {}

#[constant]
pub const SOL_POOL_PDA_SEED: &[u8] = b"sol_pool_pda";

pub fn compress_or_decompress_lamports<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    inputs: &'a InstructionDataInvoke,
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
) -> Result<()> {
    if inputs.is_compress {
        compress_lamports(inputs, ctx)
    } else {
        decompress_lamports(inputs, ctx)
    }
}

pub fn decompress_lamports<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    inputs: &'a InstructionDataInvoke,
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
) -> Result<()> {
    let recipient = match ctx.accounts.get_decompression_recipient().as_ref() {
        Some(decompression_recipient) => decompression_recipient.to_account_info(),
        None => return err!(SystemProgramError::DecompressRecipientUndefinedForDecompressSol),
    };
    let sol_pool_pda = match ctx.accounts.get_sol_pool_pda().as_ref() {
        Some(sol_pool_pda) => sol_pool_pda.to_account_info(),
        None => return err!(SystemProgramError::CompressedSolPdaUndefinedForDecompressSol),
    };
    let lamports = match inputs.compress_or_decompress_lamports {
        Some(lamports) => lamports,
        None => return err!(SystemProgramError::DeCompressLamportsUndefinedForDecompressSol),
    };

    transfer_lamports(&sol_pool_pda, &recipient, lamports)
}

pub fn compress_lamports<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info> + Bumps,
>(
    inputs: &'a InstructionDataInvoke,
    ctx: &'a Context<'a, 'b, 'c, 'info, A>,
) -> Result<()> {
    let recipient = match ctx.accounts.get_sol_pool_pda().as_ref() {
        Some(sol_pool_pda) => sol_pool_pda.to_account_info(),
        None => return err!(SystemProgramError::CompressedSolPdaUndefinedForCompressSol),
    };
    let lamports = match inputs.compress_or_decompress_lamports {
        Some(lamports) => lamports,
        None => return err!(SystemProgramError::DeCompressLamportsUndefinedForCompressSol),
    };

    transfer_lamports_cpi(
        &ctx.accounts.get_fee_payer().to_account_info(),
        &recipient,
        lamports,
    )
}

pub fn transfer_lamports<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    lamports: u64,
) -> Result<()> {
    let instruction =
        anchor_lang::solana_program::system_instruction::transfer(from.key, to.key, lamports);
    let (_, bump) =
        anchor_lang::prelude::Pubkey::find_program_address(&[SOL_POOL_PDA_SEED], &crate::ID);
    let bump = &[bump];
    let seeds = &[&[SOL_POOL_PDA_SEED, bump][..]];
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        &[from.clone(), to.clone()],
        seeds,
    )?;
    Ok(())
}
