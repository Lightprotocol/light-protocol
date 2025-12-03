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

const SYSTEM_ACCOUNTS_OFFSET: usize = 7;

/// Process transfer2 with ATA as compressed-token owner.
#[profile]
pub fn process_transfer2_with_ata(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    // Parse suffix: [transfer2_data...] ++ [wallet_idx, mint_idx, ata_idx, bump, use_delegate]
    let len = instruction_data.len();
    if len < 5 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let (transfer2_data, suffix) = instruction_data.split_at(len - 5);
    let (wallet_index, mint_index, ata_index) =
        (suffix[0] as usize, suffix[1] as usize, suffix[2] as usize);
    let (ata_bump, use_delegate) = (suffix[3], suffix[4] != 0);

    let owner_wallet = accounts
        .get(wallet_index)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let mint = accounts
        .get(mint_index)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let ata_account = accounts
        .get(ata_index)
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    // CHECK: ATA is derived correctly
    let seeds: [&[u8]; 3] = [
        owner_wallet.key().as_ref(),
        LIGHT_CPI_SIGNER.program_id.as_ref(),
        mint.key().as_ref(),
    ];
    let derived_ata =
        pinocchio_pubkey::derive_address(&seeds, Some(ata_bump), &LIGHT_CPI_SIGNER.program_id);
    if *ata_account.key() != derived_ata {
        msg!("ATA derivation mismatch");
        return Err(ProgramError::InvalidAccountData);
    }

    let (parsed, _) = CompressedTokenInstructionDataTransfer2::zero_copy_at(transfer2_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;
    let ata_packed_idx = ata_index.saturating_sub(SYSTEM_ACCOUNTS_OFFSET);

    if use_delegate {
        let first_input = parsed
            .in_token_data
            .first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        if !first_input.has_delegate() {
            msg!("delegate mode but input has no delegate");
            return Err(ProgramError::InvalidAccountData);
        }
        let delegate_packed_idx = first_input.delegate as usize;
        let delegate = accounts
            .get(delegate_packed_idx + SYSTEM_ACCOUNTS_OFFSET)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;

        // CHECK: signer
        check_signer(delegate).map_err(|_| {
            msg!("delegate not signer");
            ProgramError::MissingRequiredSignature
        })?;

        // CHECK: same delegate and compressed owner = ATA
        for input in parsed.in_token_data.iter() {
            if input.owner as usize != ata_packed_idx
                || !input.has_delegate()
                || input.delegate as usize != delegate_packed_idx
            {
                msg!("input owner/delegate mismatch");
                return Err(ProgramError::InvalidAccountData);
            }
        }
    } else {
        // CHECK: signer
        check_signer(owner_wallet).map_err(|_| {
            msg!("owner not signer");
            ProgramError::MissingRequiredSignature
        })?;

        // CHECK: compressed owner = ATA
        for input in parsed.in_token_data.iter() {
            if input.owner as usize != ata_packed_idx {
                msg!("input owner mismatch");
                return Err(ProgramError::InvalidAccountData);
            }
        }
    }

    // self-CPI with ATA as signer
    let mut ix_data = Vec::with_capacity(1 + transfer2_data.len());
    ix_data.push(101u8);
    ix_data.extend_from_slice(transfer2_data);

    let account_metas: Vec<_> = accounts
        .iter()
        .enumerate()
        .map(|(i, acc)| {
            AccountMeta::new(
                acc.key(),
                acc.is_writable(),
                acc.is_signer() || i == ata_index,
            )
        })
        .collect();

    let bump_seed = [ata_bump];
    let ata_seeds = [
        Seed::from(owner_wallet.key().as_ref()),
        Seed::from(LIGHT_CPI_SIGNER.program_id.as_ref()),
        Seed::from(mint.key().as_ref()),
        Seed::from(bump_seed.as_ref()),
    ];

    slice_invoke_signed(
        &Instruction {
            program_id: &LIGHT_CPI_SIGNER.program_id,
            accounts: &account_metas,
            data: &ix_data,
        },
        accounts,
        &[Signer::from(&ata_seeds)],
    )
    .map_err(|_| {
        msg!("self-CPI failed");
        ProgramError::InvalidArgument
    })
}
