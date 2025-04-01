use aligned_sized::*;
// use anchor_lang::{
//     prelude::*,
//     solana_program::{account_info::AccountInfo, pubkey::Pubkey},
//     Bumps,
// };
use crate::utils::transfer_lamports_cpi;
use light_compressed_account::instruction_data::zero_copy::ZInstructionDataInvoke;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    pubkey::Pubkey,
};

use crate::{
    account_traits::{InvokeAccounts, SignerAccounts},
    errors::SystemProgramError,
    LightContext,
};

// #[account]
#[aligned_sized(anchor)]
pub struct CompressedSolPda {}

pub const SOL_POOL_PDA_SEED: &[u8] = b"sol_pool_pda";

pub fn compress_or_decompress_lamports<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info>,
>(
    inputs: &'a ZInstructionDataInvoke<'a>,
    ctx: &'a A,
) -> crate::Result<()> {
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
    A: InvokeAccounts<'info> + SignerAccounts<'info>,
>(
    inputs: &'a ZInstructionDataInvoke<'a>,
    ctx: &'a A,
) -> crate::Result<()> {
    let recipient = match ctx.get_decompression_recipient() {
        Some(decompression_recipient) => decompression_recipient,
        None => {
            return Err(SystemProgramError::DecompressRecipientUndefinedForDecompressSol.into())
        }
    };
    let sol_pool_pda = match ctx.get_sol_pool_pda() {
        Some(sol_pool_pda) => sol_pool_pda,
        None => return Err(SystemProgramError::CompressedSolPdaUndefinedForDecompressSol.into()),
    };
    let lamports = match inputs.compress_or_decompress_lamports {
        Some(lamports) => lamports,
        None => return Err(SystemProgramError::DeCompressLamportsUndefinedForDecompressSol.into()),
    };

    transfer_lamports(&sol_pool_pda, &recipient, (*lamports).into())
}

pub fn compress_lamports<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info>,
>(
    inputs: &'a ZInstructionDataInvoke<'a>,
    ctx: &'a A,
) -> crate::Result<()> {
    let recipient = match ctx.get_sol_pool_pda() {
        Some(sol_pool_pda) => sol_pool_pda,
        None => return Err(SystemProgramError::CompressedSolPdaUndefinedForCompressSol.into()),
    };
    let lamports = match inputs.compress_or_decompress_lamports {
        Some(lamports) => lamports,
        None => return Err(SystemProgramError::DeCompressLamportsUndefinedForCompressSol.into()),
    };

    transfer_lamports_cpi(ctx.get_fee_payer(), &recipient, (*lamports).into())
}

pub fn transfer_lamports(from: &AccountInfo, to: &AccountInfo, lamports: u64) -> crate::Result<()> {
    let (_, bump) = pinocchio::pubkey::find_program_address(&[SOL_POOL_PDA_SEED], &crate::ID);
    let bump = &[bump];
    // Create an owned array that lives for the duration of the function
    let seed_array = [Seed::from(SOL_POOL_PDA_SEED), Seed::from(bump)];
    let signer = Signer::from(&seed_array);

    let instruction = pinocchio_system::instructions::TransferWithSeed {
        from,
        base: from,
        to,
        lamports,
        seed: "sol_pool_pda",
        owner: &crate::ID,
    };
    instruction.invoke_signed(&[signer])?;
    Ok(())
}
