use anchor_lang::prelude::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_interface::{
    instructions::extensions::{
        compressed_only::ZCompressedOnlyExtensionInstructionData, ZExtensionInstructionData,
    },
    state::{ZCTokenMut, ZExtensionStructMut},
    CTokenError,
};
use pinocchio::{account_info::AccountInfo, pubkey::pubkey_eq};
use spl_pod::solana_msg::msg;

use super::inputs::DecompressCompressOnlyInputs;

/// Validates that the destination CToken matches the source account for ATA decompress.
/// For ATA decompress (is_ata=true), verifies the destination is the correct ATA.
/// For non-ATA decompress, just validates owner matches.
///
/// # Arguments
/// * `ctoken` - Destination CToken account
/// * `destination_account` - Destination account info
/// * `input_owner` - Compressed account owner (ATA pubkey for is_ata)
/// * `wallet_owner` - Wallet owner who signs (from owner_index, only for is_ata)
/// * `ext_data` - CompressedOnly extension data
#[inline(always)]
fn validate_decompression_destination(
    ctoken: &ZCTokenMut,
    destination_account: &AccountInfo,
    input_owner: &Pubkey,
    wallet_owner: Option<&AccountInfo>,
    ext_data: &ZCompressedOnlyExtensionInstructionData,
) -> Result<(), ProgramError> {
    // Owner must match (for non-ATA) or ATA must be correctly derived (for ATA)
    if ext_data.is_ata != 0 {
        // Move to input validation and pass in instruction token data
        {
            // For ATA decompress, we need the wallet_owner
            let wallet_owner = wallet_owner.ok_or_else(|| {
                msg!("ATA decompress requires wallet_owner from owner_index");
                CTokenError::DecompressDestinationMismatch
            })?;

            // Wallet owner must be a signer
            if !wallet_owner.is_signer() {
                msg!("Wallet owner must be signer for ATA decompress");
                return Err(CTokenError::DecompressDestinationMismatch.into());
            }

            // For ATA decompress, verify the destination is the correct ATA
            // by deriving the ATA address from wallet_owner and comparing
            let wallet_owner_bytes = wallet_owner.key();
            let mint_pubkey = ctoken.base.mint.to_bytes();
            let bump = ext_data.bump;

            // ATA seeds: [wallet_owner, program_id, mint, bump]
            let bump_seed = [bump];
            let ata_seeds: [&[u8]; 4] = [
                wallet_owner_bytes.as_ref(),
                crate::LIGHT_CPI_SIGNER.program_id.as_ref(),
                mint_pubkey.as_ref(),
                bump_seed.as_ref(),
            ];

            // Derive ATA address and verify it matches the destination
            let derived_ata = pinocchio::pubkey::create_program_address(
                &ata_seeds,
                &crate::LIGHT_CPI_SIGNER.program_id,
            )
            .map_err(|_| {
                msg!("Failed to derive ATA address for decompress");
                ProgramError::InvalidSeeds
            })?;

            // Verify derived ATA matches destination account pubkey
            if !pubkey_eq(&derived_ata, destination_account.key()) {
                msg!(
                    "Decompress ATA mismatch: derived {:?} != destination {:?}",
                    solana_pubkey::Pubkey::new_from_array(derived_ata),
                    solana_pubkey::Pubkey::new_from_array(*destination_account.key())
                );
                return Err(CTokenError::DecompressDestinationMismatch.into());
            }

            // Verify the compressed account's owner (input_owner) matches the derived ATA
            // This proves the compressed account belongs to this ATA
            let input_owner_bytes = input_owner.to_bytes();
            if !pubkey_eq(&input_owner_bytes, &derived_ata) {
                msg!(
                    "Decompress ATA: compressed owner {:?} != derived ATA {:?}",
                    solana_pubkey::Pubkey::new_from_array(input_owner_bytes),
                    solana_pubkey::Pubkey::new_from_array(derived_ata)
                );
                return Err(CTokenError::DecompressDestinationMismatch.into());
            }
        }

        if !pubkey_eq(&input_owner_bytes, destination_account.key()) {
            msg!(
                "Decompress ATA mismatch: derived {:?} != destination {:?}",
                solana_pubkey::Pubkey::new_from_array(derived_ata),
                solana_pubkey::Pubkey::new_from_array(*destination_account.key())
            );
            return Err(CTokenError::DecompressDestinationMismatch.into());
        }

        // Also verify destination CToken owner matches wallet_owner
        // (destination should be wallet's ATA, owned by wallet)
        if !pubkey_eq(wallet_owner_bytes, &ctoken.base.owner.to_bytes()) {
            msg!(
                "Decompress ATA: wallet owner {:?} != destination owner {:?}",
                solana_pubkey::Pubkey::new_from_array(*wallet_owner_bytes),
                solana_pubkey::Pubkey::new_from_array(ctoken.base.owner.to_bytes())
            );
            return Err(CTokenError::DecompressDestinationMismatch.into());
        }
    } else {
        // For non-ATA decompress, owner must match
        if ctoken.base.owner.to_bytes() != input_owner.to_bytes() {
            msg!("Decompress destination owner mismatch");
            return Err(CTokenError::DecompressDestinationMismatch.into());
        }
    }

    Ok(())
}

/// Apply extension state from the input compressed account during decompress.
/// This transfers delegate, delegated_amount, and withheld_transfer_fee from
/// the compressed account's CompressedOnly extension to the CToken account.
///
/// For ATA decompress with is_ata=true, validates the destination matches the
/// derived ATA address. Existing delegate/amount on destination is preserved
/// and added to rather than overwritten.
#[inline(always)]
pub fn apply_decompress_extension_state(
    ctoken: &mut ZCTokenMut,
    destination_account: &AccountInfo,
    decompress_inputs: Option<DecompressCompressOnlyInputs>, // TODO: pass in instruction data token data
) -> Result<(), ProgramError> {
    // If no decompress inputs, nothing to transfer
    let Some(inputs) = decompress_inputs else {
        return Ok(());
    };

    // Extract CompressedOnly extension data from input TLV
    let compressed_only_data = inputs.tlv.iter().find_map(|ext| {
        if let ZExtensionInstructionData::CompressedOnly(data) = ext {
            Some(data)
        } else {
            None
        }
    });

    // If no CompressedOnly extension, nothing to transfer
    let Some(ext_data) = compressed_only_data else {
        return Ok(());
    };

    // Validate destination matches expected (ATA derivation or owner match)
    validate_decompression_destination(
        ctoken,
        destination_account,
        &Pubkey::from(*inputs.owner.key()),
        inputs.wallet_owner,
        ext_data,
    )?;

    let delegated_amount: u64 = ext_data.delegated_amount.into();
    let withheld_transfer_fee: u64 = ext_data.withheld_transfer_fee.into();

    // Handle delegate and delegated_amount
    // If destination already has delegate, skip delegate AND delegated_amount restoration (preserve existing)
    if delegated_amount > 0 || inputs.delegate.is_some() {
        let input_delegate_pubkey = inputs.delegate.map(|acc| Pubkey::from(*acc.key()));

        // Only set delegate and delegated_amount if destination doesn't already have one
        if ctoken.delegate().is_none() {
            if let Some(input_del) = input_delegate_pubkey {
                ctoken.base.set_delegate(Some(input_del))?;
            } else if delegated_amount > 0 {
                // Has delegated_amount but no delegate pubkey - invalid state
                msg!("Decompress: delegated_amount > 0 but no delegate pubkey provided");
                return Err(CTokenError::DecompressDelegatedAmountWithoutDelegate.into());
            }

            // Add delegated_amount (only when we're setting the delegate)
            if delegated_amount > 0 {
                let current = ctoken.base.delegated_amount.get();
                ctoken.base.delegated_amount.set(current + delegated_amount); // TODO: use checked_add
            }
        }
    }

    // Handle withheld_transfer_fee (always add, not overwrite)
    // Defensive: ensures compress/decompress always works for ctoken accounts.
    // It should not be possible to set withheld_transfer_fee to non-zero.
    if withheld_transfer_fee > 0 {
        let mut fee_applied = false;
        if let Some(extensions) = ctoken.extensions.as_deref_mut() {
            for extension in extensions.iter_mut() {
                if let ZExtensionStructMut::TransferFeeAccount(ref mut fee_ext) = extension {
                    fee_ext.add_withheld_amount(withheld_transfer_fee)?;
                    fee_applied = true;
                    break;
                }
            }
        }
        if !fee_applied {
            msg!("Decompress: withheld_transfer_fee > 0 but no TransferFeeAccount extension found");
            return Err(CTokenError::DecompressWithheldFeeWithoutExtension.into());
        }
    }

    // Handle is_frozen - restore frozen state from compressed token
    if ext_data.is_frozen != 0 {
        ctoken.base.set_frozen();
    }

    Ok(())
}
