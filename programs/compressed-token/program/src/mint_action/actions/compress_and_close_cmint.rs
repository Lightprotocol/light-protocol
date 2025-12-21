use anchor_compressed_token::ErrorCode;
use anchor_lang::prelude::ProgramError;
use light_ctoken_interface::{
    instructions::mint_action::ZCompressAndCloseCMintAction, state::CompressedMint,
};
use light_program_profiler::profile;
#[cfg(target_os = "solana")]
use pinocchio::sysvars::{clock::Clock, Sysvar};
use spl_pod::solana_msg::msg;

use crate::{
    mint_action::accounts::MintActionAccounts,
    shared::{convert_program_error, transfer_lamports::transfer_lamports},
};

/// Processes the CompressAndCloseCMint action by compressing and closing a CMint Solana account.
/// The compressed mint state is always preserved.
///
/// ## Process Steps
/// 1. **Idempotent Check**: If idempotent flag is set and CMint doesn't exist, succeed silently
/// 2. **State Validation**: Ensure CMint exists (cmint_decompressed = true)
/// 3. **CMint Verification**: Verify CMint account matches compressed_mint.metadata.mint
/// 4. **Extension Validation**: Ensure CMint has Compressible extension
/// 5. **Compressibility Check**: Verify is_compressible() returns true
/// 6. **Lamport Distribution**: ALL lamports -> rent_sponsor
/// 7. **Account Closure**: Assign to system program, resize to 0
/// 8. **Flag Update**: Set cmint_decompressed = false
/// 9. **Remove Compressible Extension**: Remove from compressed mint extensions
///
/// ## Note
/// CompressAndCloseCMint is **permissionless** - anyone can compress and close a CMint
/// provided is_compressible() returns true. All lamports are returned to rent_sponsor.
#[profile]
pub fn process_compress_and_close_cmint_action(
    action: &ZCompressAndCloseCMintAction,
    compressed_mint: &mut CompressedMint,
    validated_accounts: &MintActionAccounts,
) -> Result<(), ProgramError> {
    // 1. Check idempotent flag - if CMint doesn't exist and idempotent is set, succeed silently
    if action.idempotent != 0 && !compressed_mint.metadata.cmint_decompressed {
        // CMint doesn't exist, but idempotent flag is set - succeed silently
        return Ok(());
    }

    // 2. Check CMint exists (is decompressed)
    if !compressed_mint.metadata.cmint_decompressed {
        msg!("CMint does not exist (cmint_decompressed = false)");
        return Err(ErrorCode::CMintNotDecompressed.into());
    }

    let executing = validated_accounts
        .executing
        .as_ref()
        .ok_or(ErrorCode::MintActionMissingExecutingAccounts)?;

    let cmint = executing
        .cmint
        .ok_or(ErrorCode::MintActionMissingCMintAccount)?;

    let rent_sponsor = executing
        .rent_sponsor
        .ok_or(ErrorCode::MissingRentSponsor)?;

    // 3. Verify CMint account matches compressed_mint.metadata.mint
    if cmint.key() != &compressed_mint.metadata.mint.to_bytes() {
        msg!("CMint account does not match compressed_mint.metadata.mint");
        return Err(ErrorCode::InvalidCMintAccount.into());
    }

    // 4. Access compression info directly (all cmints now have embedded compression)
    let compression_info = &compressed_mint.compression;

    // 5. Verify rent_sponsor matches compression info
    if rent_sponsor.key() != &compression_info.rent_sponsor {
        msg!("Rent sponsor does not match compression info");
        return Err(ErrorCode::InvalidRentSponsor.into());
    }

    // 7. Check is_compressible (rent has expired)
    #[cfg(target_os = "solana")]
    let current_slot = Clock::get()
        .map_err(|_| ProgramError::UnsupportedSysvar)?
        .slot;
    #[cfg(not(target_os = "solana"))]
    let _current_slot = 1u64;

    #[cfg(target_os = "solana")]
    {
        let is_compressible = compression_info
            .is_compressible(cmint.data_len() as u64, current_slot, cmint.lamports())
            .map_err(|_| ProgramError::InvalidAccountData)?;

        if is_compressible.is_none() {
            msg!("CMint is not compressible (rent not expired)");
            return Err(ErrorCode::CMintNotCompressible.into());
        }
    }

    // 6. Transfer all lamports to rent_sponsor
    let cmint_lamports = cmint.lamports();
    if cmint_lamports > 0 {
        transfer_lamports(cmint_lamports, cmint, rent_sponsor).map_err(convert_program_error)?;
    }

    // 7. Close account (assign to system program, resize to 0)
    unsafe {
        cmint.assign(&[0u8; 32]);
    }
    cmint
        .resize(0)
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32 + 6000))?;

    // 8. Set cmint_decompressed = false
    compressed_mint.metadata.cmint_decompressed = false;

    // 9. Zero out compression info - only relevant when account is decompressed
    // When compressed back to a compressed account, this info should be cleared
    compressed_mint.compression = light_compressible::compression_info::CompressionInfo::default();

    Ok(())
}
