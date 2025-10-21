use anchor_lang::prelude::ProgramError;
use borsh::BorshDeserialize;
use light_account_checks::AccountIterator;
use light_compressible::config::CompressibleConfig;
use light_ctoken_types::instructions::{
    create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
    extensions::compressible::CompressibleExtensionInstructionData,
};
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, instruction::Seed, pubkey::Pubkey};
use spl_pod::solana_msg::msg;

use crate::{
    create_token_account::next_config_account,
    shared::{
        convert_program_error, create_pda_account,
        initialize_ctoken_account::initialize_ctoken_account, transfer_lamports_via_cpi,
        validate_ata_derivation,
    },
};

/// Process the create associated token account instruction (non-idempotent)
#[inline(always)]
pub fn process_create_associated_token_account(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    process_create_associated_token_account_with_mode::<false>(account_infos, instruction_data)
}

/// Process the create associated token account instruction (non-idempotent)
#[inline(always)]
pub fn process_create_associated_token_account_idempotent(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    process_create_associated_token_account_with_mode::<true>(account_infos, instruction_data)
}

/// Process create associated token account with compile-time idempotent mode
///
/// Note:
/// - we don't validate the mint because it would be very expensive with compressed mints
/// - it is possible to create an associated token account for non existing mints
/// - accounts with non existing mints can never have a balance
#[inline(always)]
#[profile]
pub(crate) fn process_create_associated_token_account_with_mode<const IDEMPOTENT: bool>(
    account_infos: &[AccountInfo],
    mut instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let instruction_inputs =
        CreateAssociatedTokenAccountInstructionData::deserialize(&mut instruction_data)
            .map_err(ProgramError::from)?;

    process_create_associated_token_account_inner::<IDEMPOTENT>(
        account_infos,
        &instruction_inputs.owner.to_bytes(),
        &instruction_inputs.mint.to_bytes(),
        instruction_inputs.bump,
        instruction_inputs.compressible_config,
    )
}

/// Core logic for creating associated token account with owner and mint as pubkeys
#[inline(always)]
#[profile]
pub(crate) fn process_create_associated_token_account_inner<const IDEMPOTENT: bool>(
    account_infos: &[AccountInfo],
    owner_bytes: &[u8; 32],
    mint_bytes: &[u8; 32],
    bump: u8,
    compressible_config: Option<CompressibleExtensionInstructionData>,
) -> Result<(), ProgramError> {
    let mut iter = AccountIterator::new(account_infos);

    let fee_payer = iter.next_signer_mut("fee_payer")?;
    let associated_token_account = iter.next_mut("associated_token_account")?;
    let _system_program = iter.next_non_mut("system_program")?;

    // If idempotent mode, check if account already exists
    if IDEMPOTENT {
        // Verify the PDA derivation is correct
        validate_ata_derivation(associated_token_account, owner_bytes, mint_bytes, bump)?;
        // If account is already owned by our program, it exists - return success
        if associated_token_account.is_owned_by(&crate::LIGHT_CPI_SIGNER.program_id) {
            return Ok(());
        }
    }

    // Check account is owned by system program (uninitialized)
    if !associated_token_account.is_owned_by(&[0u8; 32]) {
        return Err(ProgramError::IllegalOwner);
    }

    let token_account_size = if compressible_config.is_some() {
        light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
    } else {
        light_ctoken_types::BASE_TOKEN_ACCOUNT_SIZE as usize
    };

    let (compressible_config_account, custom_rent_payer) =
        if let Some(compressible_config_ix_data) = compressible_config.as_ref() {
            let (compressible_config_account, custom_rent_payer) = process_compressible_config(
                compressible_config_ix_data,
                &mut iter,
                token_account_size,
                fee_payer,
                associated_token_account,
                bump,
                owner_bytes,
                mint_bytes,
            )?;
            (Some(compressible_config_account), custom_rent_payer)
        } else {
            // Create the PDA account (with rent-exempt balance only)
            let bump_seed = [bump];
            let seeds = [
                Seed::from(owner_bytes.as_ref()),
                Seed::from(crate::LIGHT_CPI_SIGNER.program_id.as_ref()),
                Seed::from(mint_bytes.as_ref()),
                Seed::from(bump_seed.as_ref()),
            ];

            let seeds_inputs = [seeds.as_slice()];

            create_pda_account(
                fee_payer,
                associated_token_account,
                token_account_size,
                seeds_inputs,
                None,
            )?;
            (None, None)
        };

    initialize_ctoken_account(
        associated_token_account,
        mint_bytes,
        owner_bytes,
        compressible_config,
        compressible_config_account,
        custom_rent_payer,
    )?;
    Ok(())
}

#[profile]
#[allow(clippy::too_many_arguments)]
fn process_compressible_config<'info>(
    compressible_config_ix_data: &CompressibleExtensionInstructionData,
    iter: &mut AccountIterator<'info, AccountInfo>,
    token_account_size: usize,
    fee_payer: &'info AccountInfo,
    associated_token_account: &'info AccountInfo,
    ata_bump: u8,
    owner_bytes: &[u8; 32],
    mint_bytes: &[u8; 32],
) -> Result<(&'info CompressibleConfig, Option<Pubkey>), ProgramError> {
    // Validate that rent_payment is not exactly 1 epoch (footgun prevention)
    if compressible_config_ix_data.rent_payment == 1 {
        msg!("Prefunding for exactly 1 epoch is not allowed. If the account is created near an epoch boundary, it could become immediately compressible. Use 0 or 2+ epochs.");
        return Err(anchor_compressed_token::ErrorCode::OneEpochPrefundingNotAllowed.into());
    }

    if compressible_config_ix_data
        .compress_to_account_pubkey
        .is_some()
    {
        msg!("Associated token accounts must not compress to pubkey");
        return Err(ProgramError::InvalidInstructionData);
    }

    let compressible_config_account = next_config_account(iter)?;

    let rent_payer = iter.next_account("rent payer")?;

    let custom_rent_payer =
        *rent_payer.key() != compressible_config_account.rent_sponsor.to_bytes();

    let rent = compressible_config_account
        .rent_config
        .get_rent_with_compression_cost(
            token_account_size as u64,
            compressible_config_ix_data.rent_payment as u64,
        );

    // Build ATA seeds
    let ata_bump_seed = [ata_bump];
    let ata_seeds = [
        Seed::from(owner_bytes.as_ref()),
        Seed::from(crate::LIGHT_CPI_SIGNER.program_id.as_ref()),
        Seed::from(mint_bytes.as_ref()),
        Seed::from(ata_bump_seed.as_ref()),
    ];

    // Build rent sponsor seeds if needed (must be outside conditional for lifetime)
    let rent_sponsor_bump;
    let version_bytes;
    let rent_sponsor_seeds;

    // Create the PDA account (with rent-exempt balance only)
    // rent_payer will be the rent_sponsor PDA for compressible accounts
    let seeds_inputs: [&[Seed]; 2] = if custom_rent_payer {
        // Only ATA seeds when custom rent payer
        [ata_seeds.as_slice(), &[]]
    } else {
        // Both rent sponsor PDA seeds and ATA seeds
        rent_sponsor_bump = [compressible_config_account.rent_sponsor_bump];
        version_bytes = compressible_config_account.version.to_le_bytes();
        rent_sponsor_seeds = [
            Seed::from(b"rent_sponsor".as_ref()),
            Seed::from(version_bytes.as_ref()),
            Seed::from(rent_sponsor_bump.as_ref()),
        ];

        [rent_sponsor_seeds.as_slice(), ata_seeds.as_slice()]
    };

    let additional_lamports = if custom_rent_payer { Some(rent) } else { None };

    create_pda_account(
        rent_payer,
        associated_token_account,
        token_account_size,
        seeds_inputs,
        additional_lamports,
    )?;

    if !custom_rent_payer {
        // Payer transfers the additional rent (compression incentive)
        transfer_lamports_via_cpi(rent, fee_payer, associated_token_account)
            .map_err(convert_program_error)?;
    }
    Ok((
        compressible_config_account,
        if custom_rent_payer {
            Some(*rent_payer.key())
        } else {
            None
        },
    ))
}
