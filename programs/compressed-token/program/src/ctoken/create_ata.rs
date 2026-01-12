use anchor_lang::prelude::ProgramError;
use borsh::BorshDeserialize;
use light_account_checks::AccountIterator;
use light_token_interface::instructions::create_associated_token_account::CreateAssociatedTokenAccountInstructionData;
use light_program_profiler::profile;
use pinocchio::{account_info::AccountInfo, instruction::Seed};
use spl_pod::solana_msg::msg;

use crate::{
    extensions::has_mint_extensions,
    shared::{
        create_compressible_account, create_pda_account,
        initialize_ctoken_account::{initialize_ctoken_account, CTokenInitConfig},
        next_config_account, validate_ata_derivation,
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
///
/// Optional (only when compressible_config is Some):
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
        if compressible_config.compress_to_account_pubkey.is_some() {
            msg!("Associated token accounts must not compress to pubkey");
            return Err(ProgramError::InvalidInstructionData);
        }
        if compressible_config.compression_only == 0 {
            msg!("Associated token accounts must have compression_only set");
            return Err(anchor_compressed_token::ErrorCode::AtaRequiresCompressionOnly.into());
        }

        let config_account = next_config_account(&mut iter)?;
        let rent_payer = iter.next_mut("rent_payer")?;

        Some(create_compressible_account(
            compressible_config,
            &mint_extensions,
            config_account,
            rent_payer,
            associated_token_account,
            fee_payer,
            Some(ata_seeds.as_slice()), // ATA is a PDA
            true,                       // is_ata = true
        )?)
    } else {
        // Non-compressible path: fee_payer pays for account creation directly
        // Non-compressible accounts have no extensions (base 165-byte SPL layout)
        let account_size = light_token_interface::BASE_TOKEN_ACCOUNT_SIZE as usize;

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
            owner: owner_bytes,
            compressible,
            mint_extensions,
            mint_account: mint,
        },
    )
}
