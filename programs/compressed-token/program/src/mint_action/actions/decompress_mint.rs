use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_compressible::{compression_info::CompressionInfo, rent::RentConfig};
use light_ctoken_interface::{
    instructions::mint_action::ZDecompressMintAction,
    state::{CompressedMint, ExtensionStruct},
    COMPRESSED_MINT_SEED,
};
use light_program_profiler::profile;
#[cfg(target_os = "solana")]
use pinocchio::sysvars::{clock::Clock, Sysvar};
use pinocchio::{account_info::AccountInfo, instruction::Seed};
use pinocchio_system::instructions::Transfer;
use spl_pod::solana_msg::msg;

use crate::{
    create_token_account::parse_config_account,
    mint_action::accounts::MintActionAccounts,
    shared::{
        convert_program_error,
        create_pda_account::{create_pda_account, verify_pda},
    },
};

/// Processes the DecompressMint action by creating a CMint Solana account
/// from a compressed mint.
///
/// ## Process Steps
/// 1. **State Validation**: Ensure mint is not already decompressed
/// 2. **Rent Payment Validation**: rent_payment must be >= 2 (CMint is always compressible)
/// 3. **Config Validation**: Validate CompressibleConfig account
/// 4. **Write Top-Up Validation**: write_top_up must not exceed max_top_up
/// 5. **Add Compressible Extension**: Add CompressionInfo to the compressed mint extensions
/// 6. **PDA Verification**: Verify CMint account matches expected PDA derivation
/// 7. **Account Creation**: rent_sponsor pays rent exemption, fee_payer pays Light rent
/// 8. **Flag Update**: Set cmint_decompressed flag (synced at end of MintAction)
///
/// ## Note
/// DecompressMint is **permissionless** - anyone can call it (they pay for the CMint creation).
/// The authority signer is still required for MintAction, but does not need to match mint_authority.
///
/// ## Note
/// The CMint account data is NOT serialized here. The sync logic at the end
/// of the MintAction processor will write the output compressed mint to the
/// CMint account.
#[profile]
pub fn process_decompress_mint_action(
    action: &ZDecompressMintAction,
    compressed_mint: &mut CompressedMint,
    validated_accounts: &MintActionAccounts,
    mint_signer: &AccountInfo,
    fee_payer: &AccountInfo,
) -> Result<(), ProgramError> {
    // NOTE: DecompressMint is permissionless - anyone can decompress (they pay for the account)
    // No authority check required

    // 1. Check not already decompressed
    if compressed_mint.metadata.cmint_decompressed {
        msg!("CMint account already exists");
        return Err(ErrorCode::CMintAlreadyExists.into());
    }

    // 2. Validate rent_payment (CMint is ALWAYS compressible)
    // rent_payment == 0 is rejected - CMint must be compressible
    if action.rent_payment == 0 {
        msg!("rent_payment must be >= 2 (CMint is always compressible)");
        return Err(ErrorCode::InvalidRentPayment.into());
    }
    // rent_payment == 1 is rejected - epoch boundary edge case
    if action.rent_payment == 1 {
        msg!("Prefunding for exactly 1 epoch is not allowed. Use 2+ epochs.");
        return Err(ErrorCode::OneEpochPrefundingNotAllowed.into());
    }

    let executing = validated_accounts
        .executing
        .as_ref()
        .ok_or(ErrorCode::MintActionMissingExecutingAccounts)?;

    let cmint = executing
        .cmint
        .ok_or(ErrorCode::MintActionMissingCMintAccount)?;

    // 3. Get and validate CompressibleConfig account
    let config_account = executing
        .compressible_config
        .ok_or(ErrorCode::MissingCompressibleConfig)?;

    let config = parse_config_account(config_account)?;

    // 4. Validate rent_payment doesn't exceed max_funded_epochs
    if action.rent_payment > config.rent_config.max_funded_epochs {
        msg!(
            "rent_payment {} exceeds max_funded_epochs {}",
            action.rent_payment,
            config.rent_config.max_funded_epochs
        );
        return Err(ErrorCode::RentPaymentExceedsMax.into());
    }

    // 5. Validate write_top_up doesn't exceed max_top_up
    if action.write_top_up > config.rent_config.max_top_up as u32 {
        msg!(
            "write_top_up {} exceeds max_top_up {}",
            action.write_top_up,
            config.rent_config.max_top_up
        );
        return Err(ErrorCode::WriteTopUpExceedsMaximum.into());
    }

    // 6. Get rent_sponsor and verify it matches config
    let rent_sponsor = executing
        .rent_sponsor
        .ok_or(ErrorCode::MissingRentSponsor)?;

    if rent_sponsor.key() != &config.rent_sponsor.to_bytes() {
        msg!("Rent sponsor account does not match config");
        return Err(ErrorCode::InvalidRentSponsor.into());
    }

    // 7. Get current slot for last_claimed_slot
    #[cfg(target_os = "solana")]
    let current_slot = Clock::get()
        .map_err(|_| ProgramError::UnsupportedSysvar)?
        .slot;
    #[cfg(not(target_os = "solana"))]
    let current_slot = 1u64;

    // 8. Build Compressible extension and add to compressed_mint
    // NOTE: Compressible will be stripped when writing to compressed account,
    // but kept when writing to CMint (sync in mint_output.rs)
    let compression_info = CompressionInfo {
        config_account_version: config.version,
        compress_to_pubkey: 0, // Not applicable for CMint
        account_version: 3,    // ShaFlat version
        lamports_per_write: action.write_top_up.into(),
        compression_authority: config.compression_authority.to_bytes(),
        rent_sponsor: config.rent_sponsor.to_bytes(),
        last_claimed_slot: current_slot,
        rent_config: RentConfig {
            base_rent: config.rent_config.base_rent,
            compression_cost: config.rent_config.compression_cost,
            lamports_per_byte_per_epoch: config.rent_config.lamports_per_byte_per_epoch,
            max_funded_epochs: config.rent_config.max_funded_epochs,
            max_top_up: config.rent_config.max_top_up,
        },
    };

    // Add Compressible extension to compressed_mint
    let extension = ExtensionStruct::Compressible(compression_info);
    if let Some(ref mut extensions) = compressed_mint.extensions {
        extensions.push(extension);
    } else {
        compressed_mint.extensions = Some(vec![extension]);
    }

    // 9. Verify PDA derivation
    let seeds: [&[u8]; 2] = [COMPRESSED_MINT_SEED, mint_signer.key()];
    verify_pda(
        cmint.key(),
        &seeds,
        action.cmint_bump,
        &crate::LIGHT_CPI_SIGNER.program_id,
    )?;

    // 10. Calculate account size AFTER adding extension (using borsh serialization)
    let account_size = borsh::to_vec(compressed_mint)
        .map_err(|_| ErrorCode::MintActionOutputSerializationFailed)?
        .len();

    // 11. Calculate Light Protocol rent (base_rent + bytes * lamports_per_byte * epochs + compression_cost)
    let light_rent = config
        .rent_config
        .get_rent_with_compression_cost(account_size as u64, action.rent_payment as u64);

    // 12. Build seeds for rent_sponsor PDA (to sign the transfer)
    let version_bytes = config.version.to_le_bytes();
    let rent_sponsor_bump_bytes = [config.rent_sponsor_bump];
    let rent_sponsor_seeds = [
        Seed::from(b"rent_sponsor".as_ref()),
        Seed::from(version_bytes.as_ref()),
        Seed::from(rent_sponsor_bump_bytes.as_ref()),
    ];

    // 13. Build seeds for CMint PDA
    let cmint_bump_bytes = [action.cmint_bump];
    let cmint_seeds = [
        Seed::from(COMPRESSED_MINT_SEED),
        Seed::from(mint_signer.key()),
        Seed::from(cmint_bump_bytes.as_ref()),
    ];

    // 14. Create CMint PDA account
    // rent_sponsor pays ONLY the rent exemption (minimum_balance)
    // additional_lamports = None means create_pda_account only pays rent exemption
    create_pda_account(
        rent_sponsor,                        // payer: rent_sponsor PDA
        cmint,                               // account being created
        account_size,                        // size
        Some(rent_sponsor_seeds.as_slice()), // payer_seeds: rent_sponsor is PDA
        Some(cmint_seeds.as_slice()),        // account_seeds: CMint is PDA
        None,                                // rent_sponsor pays ONLY rent exemption
    )?;

    // 15. fee_payer pays the Light Protocol rent
    Transfer {
        from: fee_payer,
        to: cmint,
        lamports: light_rent,
    }
    .invoke()
    .map_err(convert_program_error)?;

    // 16. Set the cmint_decompressed flag (will be persisted in sync)
    compressed_mint.metadata.cmint_decompressed = true;

    // NOTE: Don't serialize here - the sync logic at the end of MintAction
    // processor will write the output compressed mint to CMint account

    Ok(())
}
