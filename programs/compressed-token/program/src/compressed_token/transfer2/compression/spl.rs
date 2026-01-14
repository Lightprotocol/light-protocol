use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_program_profiler::profile;
use light_sdk_types::CPI_AUTHORITY_PDA_SEED;
use light_token_interface::{
    instructions::transfer2::{ZCompression, ZCompressionMode},
    is_valid_spl_interface_pda,
};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Seed, Signer},
    msg,
};

use super::validate_compression_mode_fields;
use crate::{constants::BUMP_CPI_AUTHORITY, shared::convert_pinocchio_token_error};

/// Process compression/decompression for SPL token accounts
#[profile]
pub(super) fn process_spl_compressions(
    compression: &ZCompression,
    token_program: &[u8; 32],
    token_account_info: &AccountInfo,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    cpi_authority: &AccountInfo,
    is_restricted: bool,
) -> Result<(), ProgramError> {
    let mode = &compression.mode;

    validate_compression_mode_fields(compression)?;

    let mint_account_info =
        packed_accounts.get_u8(compression.mint, "process_spl_compression: token mint")?;
    let mint_account = *mint_account_info.key();

    let decimals = compression.decimals;

    let token_pool_account_info = packed_accounts.get_u8(
        compression.pool_account_index,
        "process_spl_compression: token pool account",
    )?;
    if !is_valid_spl_interface_pda(
        &mint_account,
        &solana_pubkey::Pubkey::new_from_array(*token_pool_account_info.key()),
        compression.pool_index,
        Some(compression.bump),
        is_restricted,
    ) {
        return Err(ErrorCode::InvalidTokenPoolPda.into());
    }
    match mode {
        ZCompressionMode::Compress => {
            let authority = packed_accounts.get_u8(
                compression.authority,
                "process_spl_compression: authority account",
            )?;
            spl_token_transfer_checked_invoke(
                token_program,
                token_account_info,
                mint_account_info,
                token_pool_account_info,
                authority,
                u64::from(*compression.amount),
                decimals,
            )?;
        }
        ZCompressionMode::Decompress => spl_token_transfer_checked_invoke_cpi(
            token_program,
            token_pool_account_info,
            mint_account_info,
            token_account_info,
            cpi_authority,
            u64::from(*compression.amount),
            decimals,
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
fn spl_token_transfer_checked_invoke_cpi(
    token_program: &[u8; 32],
    from: &AccountInfo,
    mint: &AccountInfo,
    to: &AccountInfo,
    cpi_authority: &AccountInfo,
    amount: u64,
    decimals: u8,
) -> Result<(), ProgramError> {
    let bump_seed = [BUMP_CPI_AUTHORITY];
    let seed_array = [
        Seed::from(CPI_AUTHORITY_PDA_SEED),
        Seed::from(bump_seed.as_slice()),
    ];
    let signer = Signer::from(&seed_array);

    spl_token_transfer_checked_common(
        token_program,
        from,
        mint,
        to,
        cpi_authority,
        amount,
        decimals,
        Some(&[signer]),
    )
}

#[profile]
#[inline(always)]
fn spl_token_transfer_checked_invoke(
    program_id: &[u8; 32],
    from: &AccountInfo,
    mint: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
    decimals: u8,
) -> Result<(), ProgramError> {
    spl_token_transfer_checked_common(
        program_id, from, mint, to, authority, amount, decimals, None,
    )
}

/// Performs a transfer_checked CPI to the token program.
/// transfer_checked is required for Token 2022 mints with TransferFeeConfig extension.
/// Account order: source, mint, destination, authority
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn spl_token_transfer_checked_common(
    token_program: &[u8; 32],
    from: &AccountInfo,
    mint: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    amount: u64,
    decimals: u8,
    signers: Option<&[pinocchio::instruction::Signer]>,
) -> Result<(), ProgramError> {
    // TransferChecked instruction data: discriminator (1) + amount (8) + decimals (1) = 10 bytes
    let mut instruction_data = [0u8; 10];
    instruction_data[0] = 12u8; // TransferChecked instruction discriminator
    instruction_data[1..9].copy_from_slice(&amount.to_le_bytes());
    instruction_data[9] = decimals;

    // Account order for TransferChecked: source, mint, destination, authority
    let account_metas = [
        AccountMeta::new(from.key(), true, false),
        AccountMeta::new(mint.key(), false, false), // mint is not writable
        AccountMeta::new(to.key(), true, false),
        AccountMeta::new(authority.key(), false, true),
    ];

    let instruction = pinocchio::instruction::Instruction {
        program_id: token_program,
        accounts: &account_metas,
        data: &instruction_data,
    };

    let account_infos = &[from, mint, to, authority];

    match signers {
        Some(signers) => {
            pinocchio::cpi::slice_invoke_signed(&instruction, account_infos, signers)
                .map_err(convert_pinocchio_token_error)?;
        }
        None => {
            pinocchio::cpi::slice_invoke(&instruction, account_infos)
                .map_err(convert_pinocchio_token_error)?;
        }
    }

    Ok(())
}
