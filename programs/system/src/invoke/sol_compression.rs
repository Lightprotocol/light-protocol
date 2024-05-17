use crate::{
    errors::CompressedPdaError,
    sdk::accounts::{InvokeAccounts, SignerAccounts},
    InstructionDataInvoke,
};
use account_compression::transfer_lamports_cpi;
use aligned_sized::*;
use anchor_lang::{
    prelude::*,
    solana_program::{account_info::AccountInfo, pubkey::Pubkey},
    Bumps,
};

#[account]
#[aligned_sized(anchor)]
pub struct CompressedSolPda {}

#[constant]
pub const COMPRESSED_SOL_PDA_SEED: &[u8] = b"compressed_sol_pda";

pub fn compression_lamports<
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
    } else if inputs.compression_lamports.is_some() {
        decompress_lamports(inputs, ctx)
    } else {
        Ok(())
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
    let recipient = match ctx.accounts.get_compression_recipient().as_ref() {
        Some(compression_recipient) => compression_recipient.to_account_info(),
        None => return err!(CompressedPdaError::DecompressRecipientUndefinedForDecompressSol),
    };
    let compressed_sol_pda = match ctx.accounts.get_compressed_sol_pda().as_ref() {
        Some(compressed_sol_pda) => compressed_sol_pda.to_account_info(),
        None => return err!(CompressedPdaError::CompressedSolPdaUndefinedForDecompressSol),
    };
    let lamports = match inputs.compression_lamports {
        Some(lamports) => lamports,
        None => return err!(CompressedPdaError::DeCompressLamportsUndefinedForDecompressSol),
    };

    transfer_lamports(&compressed_sol_pda, &recipient, lamports)?;

    Ok(())
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
    let recipient = match ctx.accounts.get_compressed_sol_pda().as_ref() {
        Some(compressed_sol_pda) => compressed_sol_pda.to_account_info(),
        None => return err!(CompressedPdaError::CompressedSolPdaUndefinedForCompressSol),
    };
    let lamports = match inputs.compression_lamports {
        Some(lamports) => lamports,
        None => return err!(CompressedPdaError::DeCompressLamportsUndefinedForCompressSol),
    };

    transfer_lamports_cpi(
        &ctx.accounts.get_authority().to_account_info(),
        &recipient,
        lamports,
    )
}

// pub fn transfer_lamports_compress<'info>(
//     from: &AccountInfo<'info>,
//     to: &AccountInfo<'info>,
//     lamports: u64,
// ) -> Result<()> {
//     let instruction =
//         anchor_lang::solana_program::system_instruction::transfer(from.key, to.key, lamports);
//     anchor_lang::solana_program::program::invoke(&instruction, &[from.clone(), to.clone()])?;
//     Ok(())
// }

pub fn transfer_lamports<'info>(
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    lamports: u64,
) -> Result<()> {
    let instruction =
        anchor_lang::solana_program::system_instruction::transfer(from.key, to.key, lamports);
    let (_, bump) =
        anchor_lang::prelude::Pubkey::find_program_address(&[COMPRESSED_SOL_PDA_SEED], &crate::ID);
    let bump = &[bump];
    let seeds = &[&[COMPRESSED_SOL_PDA_SEED, bump][..]];
    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        &[from.clone(), to.clone()],
        seeds,
    )?;
    Ok(())
}
