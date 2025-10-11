use aligned_sized::*;
use light_compressed_account::instruction_data::traits::InstructionData;
use light_program_profiler::profile;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
};

use crate::{
    accounts::account_traits::{InvokeAccounts, SignerAccounts},
    context::WrappedInstructionData,
    errors::SystemProgramError,
    utils::transfer_lamports_invoke,
};

#[aligned_sized(anchor)]
pub struct CompressedSolPda {}

pub const SOL_POOL_PDA_BUMP: u8 = 255;
pub const SOL_POOL_PDA_SEED: &[u8] = b"sol_pool_pda";

#[profile]
pub fn compress_or_decompress_lamports<
    'a,
    'b,
    'c: 'info,
    'info,
    A: InvokeAccounts<'info> + SignerAccounts<'info>,
    T: InstructionData<'a>,
>(
    inputs: &WrappedInstructionData<'a, T>,
    ctx: &A,
) -> crate::Result<()> {
    if inputs.compress_or_decompress_lamports().is_some() {
        if inputs.is_compress() && ctx.get_decompression_recipient()?.is_some() {
            return Err(SystemProgramError::DecompressionRecipientDefined.into());
        }
        let decompression_lamports = inputs.compress_or_decompress_lamports();
        if inputs.is_compress() {
            compress_lamports(decompression_lamports, ctx)?;
        } else {
            decompress_lamports(decompression_lamports, ctx)?;
        }
    } else if ctx.get_decompression_recipient()?.is_some() {
        return Err(SystemProgramError::DecompressionRecipientDefined.into());
    } else if ctx.get_sol_pool_pda()?.is_some() {
        return Err(SystemProgramError::SolPoolPdaDefined.into());
    }
    Ok(())
}

#[profile]
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
    let recipient = match ctx.get_decompression_recipient()? {
        Some(decompression_recipient) => decompression_recipient,
        None => {
            return Err(SystemProgramError::DecompressRecipientUndefinedForDecompressSol.into())
        }
    };
    let sol_pool_pda = match ctx.get_sol_pool_pda()? {
        Some(sol_pool_pda) => sol_pool_pda,
        None => return Err(SystemProgramError::CompressedSolPdaUndefinedForDecompressSol.into()),
    };
    let lamports = match decompression_lamports {
        Some(lamports) => lamports,
        None => return Err(SystemProgramError::DeCompressLamportsUndefinedForDecompressSol.into()),
    };

    transfer_lamports(sol_pool_pda, recipient, lamports)
}

#[profile]
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
    let recipient = match ctx.get_sol_pool_pda()? {
        Some(sol_pool_pda) => sol_pool_pda,
        None => return Err(SystemProgramError::CompressedSolPdaUndefinedForCompressSol.into()),
    };
    let lamports = match decompression_lamports {
        Some(lamports) => lamports,
        None => return Err(SystemProgramError::DecompressLamportsUndefinedForCompressSol.into()),
    };

    transfer_lamports_invoke(ctx.get_fee_payer(), recipient, lamports)
}

#[profile]
pub fn transfer_lamports(from: &AccountInfo, to: &AccountInfo, lamports: u64) -> crate::Result<()> {
    let bump = &[SOL_POOL_PDA_BUMP];
    let seed_array = [Seed::from(SOL_POOL_PDA_SEED), Seed::from(bump)];
    let signer = Signer::from(&seed_array);
    let instruction = pinocchio_system::instructions::Transfer { from, to, lamports };
    instruction.invoke_signed(&[signer])
}

#[cfg(test)]
mod test {
    use solana_pubkey::Pubkey;

    use super::*;

    fn check_hardcoded_bump(program_id: Pubkey, seeds: &[&[u8]], bump: u8) -> bool {
        let (_, found_bump) = Pubkey::find_program_address(seeds, &program_id);
        found_bump == bump
    }

    #[test]
    fn test_check_anchor_option_sol_pool_pda() {
        assert!(check_hardcoded_bump(
            crate::ID.into(),
            &[SOL_POOL_PDA_SEED],
            SOL_POOL_PDA_BUMP
        ));
    }
}
