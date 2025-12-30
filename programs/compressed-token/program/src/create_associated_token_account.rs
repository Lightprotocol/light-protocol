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
            initialize_ctoken_account, CTokenInitConfig, CompressibleInitData,
            CompressionInstructionData,
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
///   Optional (only when compressible_config is Some):
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

    // Check which extensions the mint has
    let mint_extensions = has_mint_extensions(mint)?;

    // Build ATA seeds (token account is always a PDA)
    let bump_seed = [bump];
    let ata_seeds = [
        Seed::from(owner_bytes.as_ref()),
        Seed::from(crate::LIGHT_CPI_SIGNER.program_id.as_ref()),
        Seed::from(mint_bytes.as_ref()),
        Seed::from(bump_seed.as_ref()),
    ];

    // Handle compressible vs non-compressible account creation
    let compressible = if let Some(compressible_config) = &inputs.compressible_config {
        // Validate that rent_payment is not exactly 1 epoch (footgun prevention)
        if compressible_config.rent_payment == 1 {
            msg!("Prefunding for exactly 1 epoch is not allowed. If the account is created near an epoch boundary, it could become immediately compressible. Use 0 or 2+ epochs.");
            return Err(anchor_compressed_token::ErrorCode::OneEpochPrefundingNotAllowed.into());
        }

        // Associated token accounts must not compress to pubkey
        if compressible_config.compress_to_account_pubkey.is_some() {
            msg!("Associated token accounts must not compress to pubkey");
            return Err(ProgramError::InvalidInstructionData);
        }

        // Parse additional accounts for compressible path
        let config_account = next_config_account(&mut iter)?;
        let rent_payer = iter.next_mut("rent_payer")?;

        // Calculate account size based on extensions (includes Compressible extension)
        let account_size = mint_extensions.calculate_account_size(true)?;

        let rent = config_account
            .rent_config
            .get_rent_with_compression_cost(account_size, compressible_config.rent_payment as u64);
        let account_size = account_size as usize;

        let custom_rent_payer = *rent_payer.key() != config_account.rent_sponsor.to_bytes();

        // Prevents setting executable accounts as rent_sponsor
        if custom_rent_payer && !rent_payer.is_signer() {
            msg!("Custom rent payer must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Build rent sponsor seeds if using rent sponsor PDA as fee_payer
        let version_bytes = config_account.version.to_le_bytes();
        let rent_sponsor_bump = [config_account.rent_sponsor_bump];
        let rent_sponsor_seeds = [
            Seed::from(b"rent_sponsor".as_ref()),
            Seed::from(version_bytes.as_ref()),
            Seed::from(rent_sponsor_bump.as_ref()),
        ];

        let fee_payer_seeds = if custom_rent_payer {
            None
        } else {
            Some(rent_sponsor_seeds.as_slice())
        };

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

        Some(CompressibleInitData {
            ix_data: CompressionInstructionData {
                compression_only: compressible_config.compression_only,
                token_account_version: compressible_config.token_account_version,
                write_top_up: compressible_config.write_top_up,
            },
            config_account,
            compress_to_pubkey: None, // ATAs must not compress to pubkey
            custom_rent_payer: if custom_rent_payer {
                Some(*rent_payer.key())
            } else {
                None
            },
        })
    } else {
        // Non-compressible path: fee_payer pays for account creation directly
        // Non-compressible accounts have no extensions (base 165-byte SPL layout)
        let account_size = light_ctoken_interface::BASE_TOKEN_ACCOUNT_SIZE as usize;

        create_pda_account(
            fee_payer,
            associated_token_account,
            account_size,
            None, // fee_payer is keypair
            Some(ata_seeds.as_slice()),
            None,
        )?;

        None
    };

    // Initialize the token account
    initialize_ctoken_account(
        associated_token_account,
        CTokenInitConfig {
            mint: mint_bytes,
            owner: owner_bytes,
            compressible,
            mint_extensions,
            mint_account: mint,
        },
    )
}
