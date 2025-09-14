use anchor_lang::prelude::ProgramError;
use borsh::BorshDeserialize;
use light_account_checks::AccountIterator;
use light_compressible::{config::CompressibleConfig, rent::get_rent_with_compression_cost};
use light_ctoken_types::instructions::{
    create_associated_token_account::CreateAssociatedTokenAccountInstructionData,
    extensions::compressible::CompressibleExtensionInstructionData,
};
use pinocchio::account_info::AccountInfo;
use spl_pod::solana_msg::msg;

use crate::{
    create_token_account::processor::transfer_lamports_via_cpi,
    shared::{
        create_pda_account, initialize_token_account::initialize_token_account,
        validate_ata_derivation, CreatePdaAccountConfig,
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
fn process_create_associated_token_account_with_mode<const IDEMPOTENT: bool>(
    account_infos: &[AccountInfo],
    mut instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let instruction_inputs =
        CreateAssociatedTokenAccountInstructionData::deserialize(&mut instruction_data)
            .map_err(ProgramError::from)?;
    let mut iter = AccountIterator::new(account_infos);

    let fee_payer = iter.next_signer_mut("fee_payer")?;
    let associated_token_account = iter.next_mut("associated_token_account")?;
    let system_program = iter.next_non_mut("system_program")?;

    let owner_bytes = instruction_inputs.owner.to_bytes();
    let mint_bytes = instruction_inputs.mint.to_bytes();

    // If idempotent mode, check if account already exists
    if IDEMPOTENT {
        // Verify the PDA derivation is correct
        validate_ata_derivation(
            associated_token_account,
            &owner_bytes,
            &mint_bytes,
            instruction_inputs.bump,
        )?;
        // If account is already owned by our program, it exists - return success
        if associated_token_account.is_owned_by(&crate::LIGHT_CPI_SIGNER.program_id) {
            return Ok(());
        }
    }

    // Check account is owned by system program (uninitialized)
    if !associated_token_account.is_owned_by(&[0u8; 32]) {
        return Err(ProgramError::IllegalOwner);
    }

    let token_account_size = if instruction_inputs.compressible_config.is_some() {
        light_ctoken_types::COMPRESSIBLE_TOKEN_ACCOUNT_SIZE as usize
    } else {
        light_ctoken_types::BASE_TOKEN_ACCOUNT_SIZE as usize
    };

    let seeds = &[
        owner_bytes.as_ref(),
        crate::LIGHT_CPI_SIGNER.program_id.as_ref(),
        mint_bytes.as_ref(),
    ];

    let config = CreatePdaAccountConfig {
        seeds,
        bump: instruction_inputs.bump,
        account_size: token_account_size,
        owner_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
        derivation_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
    };

    let compressible_config_account = if let Some(compressible_config_ix_data) =
        instruction_inputs.compressible_config.as_ref()
    {
        let compressible_config_account = process_compressible_config(
            compressible_config_ix_data,
            &mut iter,
            token_account_size,
            fee_payer,
            associated_token_account,
            system_program,
            config,
        )?;
        Some(compressible_config_account)
    } else {
        // Create the PDA account (with rent-exempt balance only)
        // fee_payer_for_create will be the rent_recipient PDA for compressible accounts
        create_pda_account(
            fee_payer,
            associated_token_account,
            system_program,
            config,
            None,
            None, // No additional lamports from PDA
        )?;
        None
    };

    initialize_token_account(
        associated_token_account,
        &mint_bytes,
        &owner_bytes,
        instruction_inputs.compressible_config,
        compressible_config_account,
    )?;

    Ok(())
}

fn process_compressible_config(
    compressible_config_ix_data: &CompressibleExtensionInstructionData,
    iter: &mut AccountIterator<AccountInfo>,
    token_account_size: usize,
    fee_payer: &AccountInfo,
    associated_token_account: &AccountInfo,
    system_program: &AccountInfo,
    config: CreatePdaAccountConfig,
) -> Result<CompressibleConfig, ProgramError> {
    if compressible_config_ix_data
        .compress_to_account_pubkey
        .is_some()
    {
        msg!("Associated token accounts must not compress to pubkey");
        return Err(ProgramError::InvalidInstructionData);
    }
    let config_account = iter.next_non_mut("compressible config")?;
    use anchor_lang::pubkey;
    use light_account_checks::checks::check_owner;
    check_owner(
        &pubkey!("Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX").to_bytes(),
        config_account,
    )?;
    let data = config_account.try_borrow_data().unwrap();

    // Skip Anchor's 8-byte discriminator and deserialize the actual CompressibleConfig
    use borsh::BorshDeserialize;
    use light_compressible::config::CompressibleConfig;
    let compressible_config_account =
        CompressibleConfig::deserialize(&mut &data[8..]).map_err(|e| {
            msg!("Failed to deserialize CompressibleConfig: {:?}", e);
            ProgramError::InvalidAccountData
        })?;

    // Get fee_payer_pda account for rent recipient (this will pay for account creation)
    let fee_payer_for_create = iter.next_account("fee payer pda")?;

    // The rent_recipient is a PDA derived as: [b"rent_recipient", version, 0]
    let version_bytes = compressible_config_account.version.to_le_bytes();
    let pda_seeds = &[b"rent_recipient".as_slice(), version_bytes.as_slice(), &[0]];

    // If compressible, set up the PDA config for the rent_recipient to pay for account creation
    let config_2 = crate::shared::CreatePdaAccountConfig {
        seeds: pda_seeds,
        bump: compressible_config_account.rent_recipient_bump,
        account_size: token_account_size,
        owner_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
        derivation_program_id: &crate::LIGHT_CPI_SIGNER.program_id,
    };

    // Create the PDA account (with rent-exempt balance only)
    // fee_payer_for_create will be the rent_recipient PDA for compressible accounts
    create_pda_account(
        fee_payer_for_create,
        associated_token_account,
        system_program,
        config_2,
        Some(config),
        None, // No additional lamports from PDA
    )?;

    let rent = get_rent_with_compression_cost(
        compressible_config_account.rent_config.min_rent as u64,
        compressible_config_account.rent_config.rent_per_byte as u64,
        token_account_size as u64,
        compressible_config_ix_data.rent_payment,
        compressible_config_account
            .rent_config
            .full_compression_incentive as u64,
    );
    msg!(
        "Calculating rent for {} bytes, {} epochs: {} lamports",
        token_account_size,
        compressible_config_ix_data.rent_payment,
        rent
    );

    // Payer transfers the additional rent (compression incentive)
    transfer_lamports_via_cpi(rent, fee_payer, associated_token_account)?;
    Ok(compressible_config_account)
}
