use aligned_sized::*;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    msg,
};

use crate::{
    accounts::account_traits::{InvokeAccounts, SignerAccounts},
    errors::SystemProgramError,
    utils::transfer_lamports_cpi,
};

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
    is_compress: bool,
    decompression_lamports: Option<u64>,
    ctx: &'a A,
) -> crate::Result<()> {
    if is_compress {
        msg!("is compress");
        compress_lamports(decompression_lamports, ctx)
    } else {
        msg!("is decompress");
        decompress_lamports(decompression_lamports, ctx)
    }
}

pub fn decompress_lamports<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info>,
>(
    decompression_lamports: Option<u64>,
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
    let lamports = match decompression_lamports {
        Some(lamports) => lamports,
        None => return Err(SystemProgramError::DeCompressLamportsUndefinedForDecompressSol.into()),
    };

    transfer_lamports(sol_pool_pda, recipient, lamports)
}

pub fn compress_lamports<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info>,
>(
    decompression_lamports: Option<u64>,
    ctx: &'a A,
) -> crate::Result<()> {
    let recipient = match ctx.get_sol_pool_pda() {
        Some(sol_pool_pda) => sol_pool_pda,
        None => return Err(SystemProgramError::CompressedSolPdaUndefinedForCompressSol.into()),
    };
    let lamports = match decompression_lamports {
        Some(lamports) => lamports,
        None => return Err(SystemProgramError::DecompressLamportsUndefinedForCompressSol.into()),
    };

    transfer_lamports_cpi(ctx.get_fee_payer(), recipient, lamports)
}

pub fn transfer_lamports(from: &AccountInfo, to: &AccountInfo, lamports: u64) -> crate::Result<()> {
    let bump = &[255];
    let seed_array = [Seed::from(SOL_POOL_PDA_SEED), Seed::from(bump)];
    let signer = Signer::from(&seed_array);
    let instruction = pinocchio_system::instructions::Transfer { from, to, lamports };
    instruction.invoke_signed(&[signer])
}
