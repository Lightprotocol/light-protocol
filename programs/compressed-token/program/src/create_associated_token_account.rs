use anchor_lang::prelude::ProgramError;
use borsh::BorshDeserialize;
use light_account_checks::AccountIterator;
use light_ctoken_interface::instructions::create_associated_token_account::CreateAssociatedTokenAccountInstructionData;
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, instruction::Seed};
use spl_pod::solana_msg::msg;

use crate::{
    create_token_account::next_config_account,
    extensions::has_mint_extensions,
    shared::{
        convert_program_error, create_pda_account,
        initialize_ctoken_account::{
            initialize_ctoken_account, CTokenInitConfig, CompressionInstructionData,
        },
        transfer_lamports_via_cpi, validate_ata_derivation,
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

/// Process the create associated token account instruction (idempotent)
#[inline(always)]
pub fn process_create_associated_token_account_idempotent(
    account_infos: &[AccountInfo],
    instruction_data: &[u8],
) -> Result<(), ProgramError> {
    process_create_associated_token_account_with_mode::<true>(account_infos, instruction_data)
}

/// Account order:
/// 0. owner (non-mut, non-signer)
/// 1. mint (non-mut, non-signer)
/// 2. fee_payer (signer, mut)
/// 3. associated_token_account (mut)
/// 4. system_program
/// 5. compressible_config
/// 6. rent_payer
#[profile]
#[inline(always)]
fn process_create_associated_token_account_with_mode<const IDEMPOTENT: bool>(
    account_infos: &[AccountInfo],
    mut instruction_data: &[u8],
) -> Result<(), ProgramError> {
    let inputs = CreateAssociatedTokenAccountInstructionData::deserialize(&mut instruction_data)
        .map_err(ProgramError::from)?;

    let mut iter = AccountIterator::new(account_infos);
    let owner = iter.next_account("owner")?;
    let mint = iter.next_account("mint")?;
    let fee_payer = iter.next_signer_mut("fee_payer")?;
    let associated_token_account = iter.next_mut("associated_token_account")?;
    let _system_program = iter.next_non_mut("system_program")?;
    let config_account = next_config_account(&mut iter)?;
    let rent_payer = iter.next_mut("rent_payer")?;

    let owner_bytes = owner.key();
    let mint_bytes = mint.key();
    let bump = inputs.bump;

    // If idempotent mode, check if account already exists
    if IDEMPOTENT {
        validate_ata_derivation(associated_token_account, owner_bytes, mint_bytes, bump)?;
        if associated_token_account.is_owned_by(&crate::LIGHT_CPI_SIGNER.program_id) {
            return Ok(());
        }
    }

    // Check account is owned by system program (uninitialized)
    if !associated_token_account.is_owned_by(&[0u8; 32]) {
        return Err(ProgramError::IllegalOwner);
    }

    // Validate that rent_payment is not exactly 1 epoch (footgun prevention)
    if inputs.rent_payment == 1 {
        msg!("Prefunding for exactly 1 epoch is not allowed. If the account is created near an epoch boundary, it could become immediately compressible. Use 0 or 2+ epochs.");
        return Err(anchor_compressed_token::ErrorCode::OneEpochPrefundingNotAllowed.into());
    }

    // Associated token accounts must not compress to pubkey
    if inputs.compressible_config.is_some() {
        msg!("Associated token accounts must not compress to pubkey");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Check which extensions the mint has
    let mint_extensions = has_mint_extensions(mint)?;

    // Calculate account size based on extensions
    let account_size = mint_extensions.calculate_account_size()?;

    let rent = config_account
        .rent_config
        .get_rent_with_compression_cost(account_size, inputs.rent_payment as u64);
    let account_size = account_size as usize;

    let custom_rent_payer = *rent_payer.key() != config_account.rent_sponsor.to_bytes();

    // Prevents setting executable accounts as rent_sponsor
    if custom_rent_payer && !rent_payer.is_signer() {
        msg!("Custom rent payer must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Build ATA seeds (token account is always a PDA)
    let bump_seed = [bump];
    let ata_seeds = [
        Seed::from(owner_bytes.as_ref()),
        Seed::from(crate::LIGHT_CPI_SIGNER.program_id.as_ref()),
        Seed::from(mint_bytes.as_ref()),
        Seed::from(bump_seed.as_ref()),
    ];

    // Build rent sponsor seeds if using rent sponsor PDA as fee_payer
    let version_bytes = config_account.version.to_le_bytes();
    let rent_sponsor_bump = [config_account.rent_sponsor_bump];
    let rent_sponsor_seeds = [
        Seed::from(b"rent_sponsor".as_ref()),
        Seed::from(version_bytes.as_ref()),
        Seed::from(rent_sponsor_bump.as_ref()),
    ];

    // fee_payer_seeds: Some for rent_sponsor PDA, None for custom keypair
    // new_account_seeds: Always Some (ATA is always a PDA)
    let fee_payer_seeds = if custom_rent_payer {
        None
    } else {
        Some(rent_sponsor_seeds.as_slice())
    };

    // Custom rent payer pays both account creation and compression incentive
    // Protocol rent sponsor only pays account creation, fee_payer pays compression incentive
    let additional_lamports = if custom_rent_payer { Some(rent) } else { None };

    // Create ATA account
    create_pda_account(
        rent_payer,
        associated_token_account,
        account_size,
        fee_payer_seeds,
        Some(ata_seeds.as_slice()),
        additional_lamports,
    )?;

    // When using protocol rent sponsor, fee_payer pays the compression incentive
    if !custom_rent_payer {
        transfer_lamports_via_cpi(rent, fee_payer, associated_token_account)
            .map_err(convert_program_error)?;
    }

    // Initialize the token account
    initialize_ctoken_account(
        associated_token_account,
        CTokenInitConfig {
            mint: mint_bytes,
            owner: owner_bytes,
            compress_to_pubkey: None, // ATAs must not compress to pubkey
            compression_ix_data: CompressionInstructionData {
                compression_only: inputs.compression_only,
                token_account_version: inputs.token_account_version,
                write_top_up: inputs.write_top_up,
            },
            compressible_config_account: config_account,
            custom_rent_payer: if custom_rent_payer {
                Some(*rent_payer.key())
            } else {
                None
            },
            mint_extensions,
            mint_account: mint,
        },
    )
}
