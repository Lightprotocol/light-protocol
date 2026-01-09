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

/// Validates that the destination CToken matches the source account for decompress.
/// ATA derivation and signer checks are done in input validation (token_input.rs).
///
/// Checks:
/// - For ATA: destination account ADDRESS == input_owner (ATA pubkey from token data)
/// - For ATA: destination CToken owner field == wallet_owner
/// - For non-ATA: destination CToken owner field == input_owner (wallet pubkey)
///
/// # Arguments
/// * `destination_account` - Destination CToken account info (for address check)
/// * `ctoken` - Destination CToken account data
/// * `input_owner` - Compressed account owner (ATA pubkey for is_ata, wallet for non-ATA)
/// * `wallet_owner` - Wallet owner (from owner_index, only for is_ata)
/// * `ext_data` - CompressedOnly extension data
#[inline(always)]
fn validate_decompression_destination(
    ctoken: &ZCTokenMut,
    destination_account: &AccountInfo,
    input_owner: &Pubkey,
    wallet_owner: Option<&AccountInfo>,
    ext_data: &ZCompressedOnlyExtensionInstructionData,
) -> Result<(), ProgramError> {
    if ext_data.is_ata != 0 {
        // For ATA decompress:
        // 1. Verify destination account ADDRESS == input_owner (ATA pubkey from token data)
        if !pubkey_eq(destination_account.key(), &input_owner.to_bytes()) {
            msg!(
                "Decompress ATA: destination address {:?} != token data owner {:?}",
                solana_pubkey::Pubkey::new_from_array(*destination_account.key()),
                solana_pubkey::Pubkey::new_from_array(input_owner.to_bytes())
            );
            return Err(CTokenError::DecompressDestinationMismatch.into());
        }

        // 2. Verify CToken owner field == wallet_owner
        let wallet_owner = wallet_owner.ok_or_else(|| {
            msg!("ATA decompress requires wallet_owner from owner_index");
            CTokenError::DecompressDestinationMismatch
        })?;

        if !pubkey_eq(wallet_owner.key(), &ctoken.base.owner.to_bytes()) {
            msg!(
                "Decompress ATA: wallet owner {:?} != destination owner field {:?}",
                solana_pubkey::Pubkey::new_from_array(*wallet_owner.key()),
                solana_pubkey::Pubkey::new_from_array(ctoken.base.owner.to_bytes())
            );
            return Err(CTokenError::DecompressDestinationMismatch.into());
        }
    } else {
        // For non-ATA decompress, CToken owner field must match input_owner (wallet pubkey)
        if !pubkey_eq(&ctoken.base.owner.to_bytes(), &input_owner.to_bytes()) {
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
/// ATA derivation validation is done in input validation (token_input.rs).
/// This validates destination matches token data owner and applies extension state.
#[inline(always)]
pub fn apply_decompress_extension_state(
    destination_account: &AccountInfo,
    ctoken: &mut ZCTokenMut,
    decompress_inputs: Option<DecompressCompressOnlyInputs>,
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

    // Validate destination matches token data owner
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
    if ext_data.is_frozen() {
        ctoken.base.set_frozen();
    }

    Ok(())
}
