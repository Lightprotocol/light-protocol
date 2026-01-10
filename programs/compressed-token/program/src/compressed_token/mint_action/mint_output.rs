use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use borsh::BorshSerialize;
use light_compressed_account::instruction_data::data::ZOutputCompressedAccountWithPackedContextMut;
use light_compressible::rent::get_rent_exemption_lamports;
use light_ctoken_interface::{
    hash_cache::HashCache, instructions::mint_action::ZMintActionCompressedInstructionData,
    state::CompressedMint,
};
use light_hasher::{sha256::Sha256BE, Hasher};
use light_program_profiler::profile;
use pinocchio::sysvars::{clock::Clock, Sysvar};
use spl_pod::solana_msg::msg;

use crate::{
    compressed_token::mint_action::{
        accounts::{AccountsConfig, MintActionAccounts},
        actions::process_actions,
        queue_indices::QueueIndices,
    },
    constants::COMPRESSED_MINT_DISCRIMINATOR,
    shared::{convert_program_error, transfer_lamports::transfer_lamports},
};

/// Processes the output compressed mint account.
/// When decompressed, writes mint data to CMint account (compressed account is empty).
#[profile]
pub fn process_output_compressed_account<'a>(
    parsed_instruction_data: &ZMintActionCompressedInstructionData,
    validated_accounts: &MintActionAccounts,
    output_compressed_accounts: &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
    hash_cache: &mut HashCache,
    queue_indices: &QueueIndices,
    mut compressed_mint: CompressedMint,
    accounts_config: &AccountsConfig,
) -> Result<(), ProgramError> {
    let (mint_account, token_accounts) = split_mint_and_token_accounts(output_compressed_accounts);

    process_actions(
        parsed_instruction_data,
        validated_accounts,
        &mut token_accounts.iter_mut(),
        hash_cache,
        queue_indices,
        &validated_accounts.packed_accounts,
        &mut compressed_mint,
    )?;

    if compressed_mint.metadata.cmint_decompressed {
        serialize_decompressed_mint(validated_accounts, accounts_config, &mut compressed_mint)?;
    }

    serialize_compressed_mint(mint_account, compressed_mint, queue_indices)
}

#[inline(always)]
fn split_mint_and_token_accounts<'a>(
    output_compressed_accounts: &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
) -> (
    &'a mut ZOutputCompressedAccountWithPackedContextMut<'a>,
    &'a mut [ZOutputCompressedAccountWithPackedContextMut<'a>],
) {
    if output_compressed_accounts.len() == 1 {
        (&mut output_compressed_accounts[0], &mut [])
    } else {
        let (mint_account, token_accounts) = output_compressed_accounts.split_at_mut(1);
        (&mut mint_account[0], token_accounts)
    }
}

fn serialize_compressed_mint<'a>(
    mint_account: &'a mut ZOutputCompressedAccountWithPackedContextMut<'a>,
    compressed_mint: CompressedMint,
    queue_indices: &QueueIndices,
) -> Result<(), ProgramError> {
    let compressed_account_data = mint_account
        .compressed_account
        .data
        .as_mut()
        .ok_or(ErrorCode::MintActionOutputSerializationFailed)?;

    let (discriminator, data_hash) = if compressed_mint.metadata.cmint_decompressed {
        if !compressed_account_data.data.is_empty() {
            msg!(
                "Data allocation for output mint account is wrong: {} (expected) != {} ",
                0,
                compressed_account_data.data.len()
            );
            return Err(ProgramError::InvalidAccountData);
        }
        // Zeroed discriminator and data hash preserve the address
        // of a closed compressed account without any data.
        ([0u8; 8], [0u8; 32])
    } else {
        let data = compressed_mint
            .try_to_vec()
            .map_err(|e| ProgramError::BorshIoError(e.to_string()))?;
        if data.len() != compressed_account_data.data.len() {
            msg!(
                "Data allocation for output mint account is wrong: {} (expected) != {}",
                data.len(),
                compressed_account_data.data.len()
            );
            return Err(ProgramError::InvalidAccountData);
        }

        compressed_account_data
            .data
            .copy_from_slice(data.as_slice());

        (
            COMPRESSED_MINT_DISCRIMINATOR,
            Sha256BE::hash(compressed_account_data.data)?,
        )
    };

    mint_account.set(
        crate::LIGHT_CPI_SIGNER.program_id.into(),
        0,
        Some(compressed_mint.metadata.compressed_address),
        queue_indices.output_queue_index,
        discriminator,
        data_hash,
    )?;
    Ok(())
}

fn serialize_decompressed_mint(
    validated_accounts: &MintActionAccounts,
    accounts_config: &AccountsConfig,
    compressed_mint: &mut CompressedMint,
) -> Result<(), ProgramError> {
    let cmint_account = validated_accounts
        .get_cmint()
        .ok_or(ErrorCode::CMintNotFound)?;

    // STEP 1: Serialize FIRST to know final size
    let serialized = compressed_mint
        .try_to_vec()
        .map_err(|_| ErrorCode::MintActionOutputSerializationFailed)?;
    let required_size = serialized.len();

    // STEP 2: Resize if needed (before lamport calculations)
    if cmint_account.data_len() != required_size {
        cmint_account
            .resize(required_size)
            .map_err(|_| ErrorCode::CMintResizeFailed)?;
    }

    // STEP 3: Calculate rent exemption deficit FIRST (based on final size)
    let num_bytes = required_size as u64;
    let current_lamports = cmint_account.lamports();
    let rent_exemption =
        get_rent_exemption_lamports(num_bytes).map_err(|_| ErrorCode::CMintRentExemptionFailed)?;

    // Only update rent_exemption_paid if new rent exemption is higher
    // (sponsor should get back what they originally paid)
    let rent_exemption_u32 = rent_exemption as u32;
    let mut deficit = 0u64;
    if rent_exemption_u32 > compressed_mint.compression.rent_exemption_paid {
        deficit = (rent_exemption_u32 - compressed_mint.compression.rent_exemption_paid) as u64;
        compressed_mint.compression.rent_exemption_paid = rent_exemption_u32;
    }

    // STEP 4: Add compressible top-up if not a fresh decompress
    if !accounts_config.has_decompress_mint_action {
        let current_slot = Clock::get().map_err(convert_program_error)?.slot;
        let top_up = compressed_mint
            .compression
            .calculate_top_up_lamports(num_bytes, current_slot, current_lamports)
            .map_err(|_| ErrorCode::CMintTopUpCalculationFailed)?;
        // Add compressible top-up to rent deficit
        deficit = deficit.saturating_add(top_up);
    }

    // STEP 5: Single unified transfer if needed
    if deficit > 0 {
        let fee_payer = validated_accounts
            .executing
            .as_ref()
            .map(|exec| exec.system.fee_payer)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        transfer_lamports(deficit, fee_payer, cmint_account).map_err(convert_program_error)?;
    }

    // STEP 6: Write serialized data
    let mut cmint_data = cmint_account
        .try_borrow_mut_data()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    cmint_data[..serialized.len()].copy_from_slice(&serialized);

    Ok(())
}
