//! Transfer2WithAta - Thin wrapper for Transfer2 operations where ALL compressed
//! token inputs have owner = ATA pubkey.
//!
//! This is for tokens compressed with compress_to_pubkey=true on an ATA.
//! For mixed ownership or wallet-owned tokens, use regular Transfer2.
//!
//! Security model:
//! - User signs with their wallet (owner_wallet)
//! - ATA is derived from [owner_wallet, program_id, mint]
//! - ALL input compressed tokens must have owner = derived ATA (enforced)
//! - User is authorizing operations on tokens they already control
//!
//! Instruction data layout: [Transfer2 instruction data] ++ [wallet_index: u8,
//! mint_index: u8, ata_index: u8, ata_bump: u8]
//!
//! The indices are ABSOLUTE positions in the accounts array.

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

/// Process the Transfer2WithAta instruction
#[profile]
pub fn process_transfer2_with_ata(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let data_len = instruction_data.len();
    if data_len < 4 {
        msg!("Transfer2WithAta: instruction data too short");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Parse indices from end of instruction data (ABSOLUTE positions)
    let wallet_index = instruction_data[data_len - 4] as usize;
    let mint_index = instruction_data[data_len - 3] as usize;
    let ata_index = instruction_data[data_len - 2] as usize;
    let ata_bump = instruction_data[data_len - 1];
    let transfer2_data = &instruction_data[..data_len - 4];

    let owner_wallet = accounts
        .get(wallet_index)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let mint = accounts
        .get(mint_index)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let ata_account = accounts
        .get(ata_index)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // CHECK 1: owner_wallet must be signer
    check_signer(owner_wallet).map_err(|e| {
        msg!("Transfer2WithAta: owner_wallet must be signer");
        ProgramError::from(e)
    })?;

    // CHECK 2: ata_account must match derived ATA
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

    // CHECK: ata is owner of all inputs
    let (parsed_transfer2, _) =
        CompressedTokenInstructionDataTransfer2::zero_copy_at(transfer2_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

    const SYSTEM_ACCOUNTS_OFFSET: usize = 7;
    let ata_packed_index = ata_index.saturating_sub(SYSTEM_ACCOUNTS_OFFSET);

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

    // Build Transfer2 instruction
    let mut transfer2_ix_data = Vec::with_capacity(1 + transfer2_data.len());
    transfer2_ix_data.push(101u8); // Transfer2 discriminator
    transfer2_ix_data.extend_from_slice(transfer2_data);

    // Build account metas - preserve order, mark ATA as signer
    // pinocchio AccountMeta::new order: (pubkey, is_writable, is_signer)
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

    // PDA signer seeds for ATA
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
