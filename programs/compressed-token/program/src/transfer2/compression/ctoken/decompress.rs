use anchor_lang::prelude::ProgramError;
use light_compressed_account::Pubkey;
use light_ctoken_interface::{
    instructions::extensions::ZExtensionInstructionData,
    state::{ZCTokenMut, ZExtensionStructMut},
    CTokenError,
};
use spl_pod::solana_msg::msg;

use super::inputs::DecompressCompressOnlyInputs;

/// Validates that the destination CToken is a fresh/zeroed account with matching owner.
/// This ensures we can recreate the exact account state from the CompressedOnly extension.
#[inline(always)]
fn validate_decompression_destination(
    ctoken: &ZCTokenMut,
    input_owner: &Pubkey,
) -> Result<(), ProgramError> {
    // Owner must match
    if ctoken.base.owner.to_bytes() != input_owner.to_bytes() {
        msg!("Decompress destination owner mismatch");
        return Err(CTokenError::DecompressDestinationNotFresh.into());
    }

    // Amount must be 0
    if ctoken.base.amount.get() != 0 {
        msg!("Decompress destination has non-zero amount");
        return Err(CTokenError::DecompressDestinationNotFresh.into());
    }

    // Must not have delegate
    if ctoken.delegate().is_some() {
        msg!("Decompress destination has delegate set");
        return Err(CTokenError::DecompressDestinationNotFresh.into());
    }

    // Delegated amount must be 0
    if ctoken.base.delegated_amount.get() != 0 {
        msg!("Decompress destination has non-zero delegated_amount");
        return Err(CTokenError::DecompressDestinationNotFresh.into());
    }

    // Must not have close authority
    if ctoken.close_authority().is_some() {
        msg!("Decompress destination has close_authority set");
        return Err(CTokenError::DecompressDestinationNotFresh.into());
    }

    Ok(())
}

/// Apply extension state from the input compressed account during decompress.
/// This transfers delegate, delegated_amount, and withheld_transfer_fee from
/// the compressed account's CompressedOnly extension to the CToken account.
#[inline(always)]
pub fn apply_decompress_extension_state(
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

    // Validate destination is a fresh account with matching owner
    validate_decompression_destination(ctoken, &Pubkey::from(*inputs.owner.key()))?;

    let delegated_amount: u64 = ext_data.delegated_amount.into();
    let withheld_transfer_fee: u64 = ext_data.withheld_transfer_fee.into();

    // Handle delegate and delegated_amount
    if delegated_amount > 0 || inputs.delegate.is_some() {
        let input_delegate_pubkey = inputs.delegate.map(|acc| Pubkey::from(*acc.key()));

        if let Some(input_del) = input_delegate_pubkey {
            // Set delegate from the input (destination is guaranteed fresh with no delegate)
            ctoken.base.set_delegate(Some(input_del))?;
        } else if delegated_amount > 0 {
            // Has delegated_amount but no delegate pubkey - invalid state
            msg!("Decompress: delegated_amount > 0 but no delegate pubkey provided");
            return Err(CTokenError::InvalidAccountData.into());
        }

        // Set delegated_amount (destination is guaranteed to have 0)
        if delegated_amount > 0 {
            ctoken.base.delegated_amount.set(delegated_amount);
        }
    }

    // Handle withheld_transfer_fee
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
            return Err(CTokenError::InvalidAccountData.into());
        }
    }

    // Handle is_frozen - restore frozen state from compressed token
    if ext_data.is_frozen != 0 {
        ctoken.base.set_frozen();
    }

    Ok(())
}
