use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_array_map::pubkey_eq;
use light_compressible::compression_info::CompressionInfo;
use light_program_profiler::profile;
use light_token_interface::{
    instructions::mint_action::ZDecompressMintAction, state::Mint, COMPRESSED_MINT_SEED,
};
use pinocchio::{
    account_info::AccountInfo,
    instruction::Seed,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
};
use pinocchio_system::instructions::Transfer;
use spl_pod::solana_msg::msg;

use crate::{
    compressed_token::mint_action::accounts::MintActionAccounts,
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
/// 2. **Rent Payment Validation**: rent_payment must be >= 2
/// 3. **Config Validation**: Validate CompressibleConfig account
/// 4. **Write Top-Up Validation**: write_top_up must not exceed max_top_up
/// 5. **Add Compressible Extension**: Add CompressionInfo to the compressed mint extensions
/// 6. **PDA Verification**: Verify CMint account matches expected PDA derivation
/// 7. **Account Creation**: rent_sponsor pays rent exemption, fee_payer pays Light rent
/// 8. **Flag Update**: Set cmint_decompressed flag
///
/// ## Note
/// DecompressMint is **permissionless** - the caller pays initial rent, rent exemption is sponsored by the rent_sponsor.
/// The authority signer is still required for MintAction, but does not need to match mint_authority.
#[profile]
pub fn process_decompress_mint_action(
    action: &ZDecompressMintAction,
    compressed_mint: &mut Mint,
    validated_accounts: &MintActionAccounts,
    fee_payer: &AccountInfo,
) -> Result<(), ProgramError> {
    // 1. Check not already decompressed
    if compressed_mint.metadata.mint_decompressed {
        msg!("CMint account already exists");
        return Err(ErrorCode::CMintAlreadyExists.into());
    }

    // 2. CMint requires at least 2 epochs of rent prepayment (always compressible)
    if action.rent_payment < 2 {
        msg!(
            "CMint requires at least 2 epochs of rent prepayment. Got {}.",
            action.rent_payment
        );
        return Err(ErrorCode::InvalidRentPayment.into());
    }

    let executing = validated_accounts
        .executing
        .as_ref()
        .ok_or(ErrorCode::MintActionMissingExecutingAccounts)?;

    let cmint = executing
        .cmint
        .ok_or(ErrorCode::MintActionMissingCMintAccount)?;

    // 3. Get CompressibleConfig (already parsed and validated as active)
    let config = executing
        .compressible_config
        .ok_or(ErrorCode::MissingCompressibleConfig)?;

    // 4. Validate write_top_up doesn't exceed max_top_up
    if action.write_top_up > config.rent_config.max_top_up as u32 {
        msg!(
            "write_top_up {} exceeds max_top_up {}",
            action.write_top_up,
            config.rent_config.max_top_up
        );
        return Err(ErrorCode::WriteTopUpExceedsMaximum.into());
    }

    // Get rent_sponsor and verify it matches config
    let rent_sponsor = executing
        .rent_sponsor
        .ok_or(ErrorCode::MissingRentSponsor)?;

    if rent_sponsor.key() != &config.rent_sponsor.to_bytes() {
        msg!("Rent sponsor account does not match config");
        return Err(ErrorCode::InvalidRentSponsor.into());
    }

    // Get current slot for last_claimed_slot
    let current_slot = Clock::get()
        .map_err(|_| ProgramError::UnsupportedSysvar)?
        .slot;

    // 5. Set compression info on compressed_mint (rent_exemption_paid set after account_size calculation)
    compressed_mint.compression = CompressionInfo {
        config_account_version: config.version,
        compress_to_pubkey: 0, // Not applicable for CMint
        account_version: 3,    // ShaFlat version
        lamports_per_write: action.write_top_up.into(),
        compression_authority: config.compression_authority.to_bytes(),
        rent_sponsor: config.rent_sponsor.to_bytes(),
        last_claimed_slot: current_slot,
        rent_exemption_paid: 0, // Updated below after account_size calculation
        _reserved: 0,
        rent_config: config.rent_config,
    };

    // 6. Verify PDA derivation using stored mint_signer from compressed_mint metadata
    let pda_mint_signer_bytes: &[u8] = compressed_mint.metadata.mint_signer.as_ref();
    let seeds: [&[u8]; 2] = [COMPRESSED_MINT_SEED, pda_mint_signer_bytes];
    let canonical_bump = verify_pda(cmint.key(), &seeds, &crate::LIGHT_CPI_SIGNER.program_id)?;
    // 6b. Verify CMint account matches compressed_mint.metadata.mint
    if !pubkey_eq(cmint.key(), &compressed_mint.metadata.mint.to_bytes()) {
        msg!("CMint account does not match compressed_mint.metadata.mint");
        return Err(ErrorCode::InvalidCMintAccount.into());
    }

    // 7. Account creation: rent_sponsor pays rent exemption, fee_payer pays Light rent
    // 7a. Calculate account size AFTER adding extension (using borsh serialization)
    let account_size = borsh::to_vec(compressed_mint)
        .map_err(|_| ErrorCode::MintActionOutputSerializationFailed)?
        .len();

    // 7a.1. Store rent exemption at creation (only query Rent sysvar here, never again)
    let rent_exemption_paid: u32 = Rent::get()
        .map_err(|_| ProgramError::UnsupportedSysvar)?
        .minimum_balance(account_size)
        .try_into()
        .map_err(|_| ProgramError::ArithmeticOverflow)?;
    compressed_mint.compression.rent_exemption_paid = rent_exemption_paid;

    // 7b. Calculate Light Protocol rent (base_rent + bytes * lamports_per_byte * epochs + compression_cost)
    let light_rent = config
        .rent_config
        .get_rent_with_compression_cost(account_size as u64, action.rent_payment as u64);

    // 7c. Build seeds for rent_sponsor PDA (to sign the transfer)
    let version_bytes = config.version.to_le_bytes();
    let rent_sponsor_bump_bytes = [config.rent_sponsor_bump];
    let rent_sponsor_seeds = [
        Seed::from(b"rent_sponsor".as_ref()),
        Seed::from(version_bytes.as_ref()),
        Seed::from(rent_sponsor_bump_bytes.as_ref()),
    ];

    // 7d. Build seeds for CMint PDA using canonical bump from verify_pda
    let cmint_bump_bytes = [canonical_bump];
    let mint_signer_bytes: &[u8] = compressed_mint.metadata.mint_signer.as_ref();
    let cmint_seeds = [
        Seed::from(COMPRESSED_MINT_SEED),
        Seed::from(mint_signer_bytes),
        Seed::from(cmint_bump_bytes.as_ref()),
    ];

    // 7e. Create CMint PDA account
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

    // 7f. fee_payer pays the Light Protocol rent
    Transfer {
        from: fee_payer,
        to: cmint,
        lamports: light_rent,
    }
    .invoke()
    .map_err(convert_program_error)?;

    // 8. Set the mint_decompressed flag
    compressed_mint.metadata.mint_decompressed = true;

    Ok(())
}
