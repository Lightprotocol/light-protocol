use anchor_compressed_token::check_spl_token_pool_derivation_with_index;
use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_ctoken_types::instructions::transfer2::{ZCompression, ZCompressionMode};
use light_program_profiler::profile;
use light_sdk_types::CPI_AUTHORITY_PDA_SEED;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Seed, Signer},
    msg,
};

use super::validate_compression_mode_fields;
use crate::constants::BUMP_CPI_AUTHORITY;

/// Process compression/decompression for SPL token accounts
#[profile]
pub(super) fn process_spl_compressions(
    compression: &ZCompression,
    token_program: &[u8; 32],
    token_account_info: &AccountInfo,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    cpi_authority: &AccountInfo,
) -> Result<(), ProgramError> {
    let mode = &compression.mode;

    validate_compression_mode_fields(compression)?;

    let mint_account = *packed_accounts
        .get_u8(compression.mint, "process_spl_compression: token mint")?
        .key();
    let token_pool_account_info = packed_accounts.get_u8(
        compression.pool_account_index,
        "process_spl_compression: token pool account",
    )?;
    check_spl_token_pool_derivation_with_index(
        &solana_pubkey::Pubkey::new_from_array(*token_pool_account_info.key()),
        &solana_pubkey::Pubkey::new_from_array(mint_account),
        compression.pool_index,
        Some(compression.bump),
    )?;
    match mode {
        ZCompressionMode::Compress => {
            let authority = packed_accounts.get_u8(
                compression.authority,
                "process_spl_compression: authority account",
            )?;
            spl_token_transfer_invoke(
                token_program,
                token_account_info,
                token_pool_account_info,
                authority,
                u64::from(*compression.amount),
            )?;
        }
        ZCompressionMode::Decompress => spl_token_transfer_invoke_cpi(
            token_program,
            token_pool_account_info,
            token_account_info,
            cpi_authority,
            u64::from(*compression.amount),
        )?,
        ZCompressionMode::CompressAndClose => {
            msg!("CompressAndClose is unimplemented for spl token accounts");
            unimplemented!()
        }
    }
    Ok(())
}

#[profile]
#[inline(always)]
fn spl_token_transfer_invoke_cpi(
    token_program: &[u8; 32],
    from: &AccountInfo,
    to: &AccountInfo,
    cpi_authority: &AccountInfo,
    amount: u64,
) -> Result<(), ProgramError> {
    msg!("spl_token_transfer_invoke_cpi");
    msg!(
        "from {:?}",
        solana_pubkey::Pubkey::new_from_array(*from.key())
    );
    msg!("to {:?}", solana_pubkey::Pubkey::new_from_array(*to.key()));
    msg!("amount {:?}", amount);
    let bump_seed = [BUMP_CPI_AUTHORITY];
    let seed_array = [
        Seed::from(CPI_AUTHORITY_PDA_SEED),
        Seed::from(bump_seed.as_slice()),
    ];
    let signer = Signer::from(&seed_array);

    spl_token_transfer_common(
        token_program,
        from,
        to,
        cpi_authority,
        amount,
        Some(&[signer]),
    )
}

#[profile]
#[inline(always)]
fn spl_token_transfer_invoke(
    program_id: &[u8; 32],
    from: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
) -> Result<(), ProgramError> {
    msg!("spl_token_transfer_invoke");
    msg!(
        "from {:?}",
        solana_pubkey::Pubkey::new_from_array(*from.key())
    );
    msg!("to {:?}", solana_pubkey::Pubkey::new_from_array(*to.key()));
    msg!("amount {:?}", amount);
    spl_token_transfer_common(program_id, from, to, authority, amount, None)
}

#[inline(always)]
fn spl_token_transfer_common(
    token_program: &[u8; 32],
    from: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
    signers: Option<&[pinocchio::instruction::Signer]>,
) -> Result<(), ProgramError> {
    let mut instruction_data = [0u8; 9];
    instruction_data[0] = 3u8; // Transfer instruction discriminator
    instruction_data[1..9].copy_from_slice(&amount.to_le_bytes());

    let account_metas = [
        AccountMeta::new(from.key(), true, false),
        AccountMeta::new(to.key(), true, false),
        AccountMeta::new(authority.key(), false, true),
    ];

    let instruction = pinocchio::instruction::Instruction {
        program_id: token_program,
        accounts: &account_metas,
        data: &instruction_data,
    };

    let account_infos = &[from, to, authority];

    match signers {
        Some(signers) => {
            pinocchio::cpi::slice_invoke_signed(&instruction, account_infos, signers)
                .map_err(|_| ProgramError::InvalidArgument)?;
        }
        None => {
            pinocchio::cpi::slice_invoke(&instruction, account_infos)
                .map_err(|_| ProgramError::InvalidArgument)?;
        }
    }

    Ok(())
}
