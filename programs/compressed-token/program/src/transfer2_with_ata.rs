use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::checks::check_signer;
use light_ctoken_types::instructions::transfer2::CompressedTokenInstructionDataTransfer2;
use light_program_profiler::profile;
use light_zero_copy::traits::ZeroCopyAt;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction, Seed, Signer},
};
use spl_pod::solana_msg::msg;

use crate::{shared::cpi::slice_invoke_signed, LIGHT_CPI_SIGNER};

const NO_DELEGATE: u8 = 255;

/// Process the Transfer2WithAta instruction
///
/// Supports two modes:
/// 1. Owner mode (delegate_index = 255): owner_wallet must be signer
/// 2. Delegate mode (delegate_index != 255): delegate must be signer AND match input delegate fields
///
/// In both modes, ATA is derived from owner_wallet + mint and becomes signer via PDA signing.
#[profile]
pub fn process_transfer2_with_ata(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse: [transfer2_data...] ++ [wallet_index, mint_index, ata_index, ata_bump, delegate_index]
    let suffix_start = instruction_data
        .len()
        .checked_sub(5)
        .ok_or(ProgramError::InvalidInstructionData)?;
    let wallet_index = *instruction_data
        .get(suffix_start)
        .ok_or(ProgramError::InvalidInstructionData)? as usize;
    let mint_index = *instruction_data
        .get(suffix_start + 1)
        .ok_or(ProgramError::InvalidInstructionData)? as usize;
    let ata_index = *instruction_data
        .get(suffix_start + 2)
        .ok_or(ProgramError::InvalidInstructionData)? as usize;
    let ata_bump = *instruction_data
        .get(suffix_start + 3)
        .ok_or(ProgramError::InvalidInstructionData)?;
    let delegate_index = *instruction_data
        .get(suffix_start + 4)
        .ok_or(ProgramError::InvalidInstructionData)?;
    let transfer2_data = &instruction_data[..suffix_start];

    let owner_wallet = accounts
        .get(wallet_index)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let mint = accounts
        .get(mint_index)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let ata_account = accounts
        .get(ata_index)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // CHECK: ATA derivation (always from owner_wallet + mint)
    let seeds: [&[u8]; 3] = [
        owner_wallet.key().as_ref(),
        LIGHT_CPI_SIGNER.program_id.as_ref(),
        mint.key().as_ref(),
    ];
    let derived_ata =
        pinocchio_pubkey::derive_address(&seeds, Some(ata_bump), &LIGHT_CPI_SIGNER.program_id);

    if *ata_account.key() != derived_ata {
        msg!("Transfer2WithAta: ATA derivation mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    // Parse transfer2 data to validate inputs
    let (parsed_transfer2, _) =
        CompressedTokenInstructionDataTransfer2::zero_copy_at(transfer2_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

    const SYSTEM_ACCOUNTS_OFFSET: usize = 7;
    let ata_packed_index = ata_index.saturating_sub(SYSTEM_ACCOUNTS_OFFSET);

    // CHECK: Signer and input validation based on mode
    if delegate_index != NO_DELEGATE {
        // Delegate mode: delegate must be signer and match all input delegate fields
        let delegate = accounts
            .get(delegate_index as usize)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;

        check_signer(delegate).map_err(|e| {
            msg!("Transfer2WithAta: delegate must be signer");
            ProgramError::from(e)
        })?;

        let delegate_packed_index =
            (delegate_index as usize).saturating_sub(SYSTEM_ACCOUNTS_OFFSET);

        for (i, input) in parsed_transfer2.in_token_data.iter().enumerate() {
            // Check owner = ATA
            if input.owner as usize != ata_packed_index {
                msg!(
                    "Transfer2WithAta: input {} owner mismatch: {} != {}",
                    i,
                    input.owner,
                    ata_packed_index
                );
                return Err(ProgramError::InvalidAccountData);
            }
            // Check delegate field matches the delegate signer
            if !input.has_delegate() || input.delegate as usize != delegate_packed_index {
                msg!(
                    "Transfer2WithAta: input {} delegate mismatch: has_delegate={}, delegate={}, expected={}",
                    i,
                    input.has_delegate(),
                    input.delegate,
                    delegate_packed_index
                );
                return Err(ProgramError::InvalidAccountData);
            }
        }
    } else {
        // Owner mode: owner_wallet must be signer
        check_signer(owner_wallet).map_err(|e| {
            msg!("Transfer2WithAta: owner_wallet must be signer");
            ProgramError::from(e)
        })?;

        // CHECK: ATA owns all inputs
        for (i, input) in parsed_transfer2.in_token_data.iter().enumerate() {
            if input.owner as usize != ata_packed_index {
                msg!(
                    "Transfer2WithAta: input {} owner mismatch: {} != {}",
                    i,
                    input.owner,
                    ata_packed_index
                );
                return Err(ProgramError::InvalidAccountData);
            }
        }
    }

    let mut transfer2_ix_data = Vec::with_capacity(1 + transfer2_data.len());
    transfer2_ix_data.push(101u8); // Transfer2 discriminator
    transfer2_ix_data.extend_from_slice(transfer2_data);

    // Build account metas, make ATA as signer.
    let mut account_metas = Vec::with_capacity(accounts.len());
    for (i, acc) in accounts.iter().enumerate() {
        let is_signer = acc.is_signer() || i == ata_index;
        account_metas.push(AccountMeta::new(acc.key(), acc.is_writable(), is_signer));
    }

    let instruction = Instruction {
        program_id: &LIGHT_CPI_SIGNER.program_id,
        accounts: &account_metas,
        data: &transfer2_ix_data,
    };

    let bump_seed = [ata_bump];
    let ata_seeds = [
        Seed::from(owner_wallet.key().as_ref()),
        Seed::from(LIGHT_CPI_SIGNER.program_id.as_ref()),
        Seed::from(mint.key().as_ref()),
        Seed::from(bump_seed.as_ref()),
    ];
    let signer = Signer::from(&ata_seeds);

    slice_invoke_signed(&instruction, accounts, &[signer]).map_err(|e| {
        msg!("Transfer2WithAta: CPI failed: {:?}", e);
        ProgramError::InvalidArgument
    })
}
