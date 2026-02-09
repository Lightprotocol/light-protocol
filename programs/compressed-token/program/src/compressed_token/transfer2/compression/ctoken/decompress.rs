use anchor_lang::prelude::ProgramError;
use light_account_checks::packed_accounts::ProgramPackedAccounts;
use light_compressed_account::Pubkey;
use light_token_interface::{
    instructions::extensions::{find_compressed_only, ZCompressedOnlyExtensionInstructionData},
    state::{ZExtensionStructMut, ZTokenMut},
    TokenError,
};
use pinocchio::{account_info::AccountInfo, pubkey::pubkey_eq};
use spl_pod::solana_msg::msg;

use super::inputs::DecompressCompressOnlyInputs;

/// Validate and apply CompressedOnly extension state from compressed account to CToken during decompress.
#[inline(always)]
pub fn validate_and_apply_compressed_only(
    destination_account: &AccountInfo,
    ctoken: &mut ZTokenMut,
    decompress_inputs: Option<DecompressCompressOnlyInputs>,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    compression_amount: u64,
) -> Result<(), ProgramError> {
    let Some(inputs) = decompress_inputs else {
        return Ok(());
    };

    let Some(ext_data) = find_compressed_only(inputs.tlv) else {
        return Ok(());
    };

    // === VALIDATE amount matches ===
    let input_amount: u64 = inputs.input_token_data.amount.into();
    if compression_amount != input_amount {
        msg!(
            "Decompress: amount mismatch (compression: {}, input: {})",
            compression_amount,
            input_amount
        );
        return Err(TokenError::DecompressAmountMismatch.into());
    }

    // === VALIDATE destination ownership ===
    let input_owner = packed_accounts.get_u8(inputs.input_token_data.owner, "input owner")?;
    validate_destination(
        ctoken,
        destination_account,
        input_owner.key(),
        ext_data,
        packed_accounts,
    )?;

    // === APPLY delegate state ===
    apply_delegate(
        ctoken,
        ext_data,
        &inputs,
        packed_accounts,
        compression_amount,
    )?;

    // === APPLY withheld fee ===
    apply_withheld_fee(ctoken, ext_data)?;

    // === APPLY frozen state ===
    if ext_data.is_frozen() {
        ctoken.base.set_frozen();
    }

    Ok(())
}

/// Validate destination matches the source account for decompress.
///
/// For non-ATA: CToken owner == input_owner (wallet pubkey)
/// For ATA: destination address == input_owner (ATA pubkey), and CToken owner == wallet_owner
#[inline(always)]
fn validate_destination(
    ctoken: &ZTokenMut,
    destination: &AccountInfo,
    input_owner_key: &[u8; 32],
    ext_data: &ZCompressedOnlyExtensionInstructionData,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
) -> Result<(), ProgramError> {
    // Non-ATA: simple owner match (handle simpler case first)
    if !ext_data.is_ata() {
        if !pubkey_eq(ctoken.base.owner.array_ref(), input_owner_key) {
            msg!("Decompress destination owner mismatch");
            return Err(TokenError::DecompressDestinationMismatch.into());
        }
        return Ok(());
    }

    // ATA: destination address == input_owner (ATA pubkey)
    if !pubkey_eq(destination.key(), input_owner_key) {
        msg!("Decompress ATA: destination address mismatch");
        return Err(TokenError::DecompressDestinationMismatch.into());
    }

    // ATA: wallet owner == CToken owner field
    let wallet = packed_accounts.get_u8(ext_data.owner_index, "wallet owner")?;
    if !pubkey_eq(wallet.key(), ctoken.base.owner.array_ref()) {
        msg!("Decompress ATA: wallet owner mismatch");
        return Err(TokenError::DecompressDestinationMismatch.into());
    }
    Ok(())
}

/// Apply delegate state. Resolves delegate only when needed (inside the check).
#[inline(always)]
fn apply_delegate(
    ctoken: &mut ZTokenMut,
    ext_data: &ZCompressedOnlyExtensionInstructionData,
    inputs: &DecompressCompressOnlyInputs,
    packed_accounts: &ProgramPackedAccounts<'_, AccountInfo>,
    compression_amount: u64,
) -> Result<(), ProgramError> {
    // Skip if destination already has delegate
    if ctoken.delegate().is_some() {
        return Ok(());
    }

    let delegated_amount: u64 = ext_data.delegated_amount.into();

    // Resolve delegate only when needed
    let input_delegate = if inputs.input_token_data.has_delegate() {
        Some(packed_accounts.get_u8(inputs.input_token_data.delegate, "delegate")?)
    } else {
        None
    };

    if let Some(delegate_acc) = input_delegate {
        ctoken
            .base
            .set_delegate(Some(Pubkey::from(*delegate_acc.key())))?;
        // Cap delegated_amount by the actual compressed balance to prevent
        // over-increasing delegation when compressed account had more delegated than balance.
        let capped = delegated_amount.min(compression_amount);
        if capped > 0 {
            let current = ctoken.base.delegated_amount.get();
            ctoken.base.delegated_amount.set(
                current
                    .checked_add(capped)
                    .ok_or(ProgramError::ArithmeticOverflow)?,
            );
        }
    } else if delegated_amount > 0 {
        msg!("Decompress: delegated_amount > 0 but no delegate");
        return Err(TokenError::DecompressDelegatedAmountWithoutDelegate.into());
    }

    Ok(())
}

/// Apply withheld transfer fee to TransferFeeAccount extension.
#[inline(always)]
fn apply_withheld_fee(
    ctoken: &mut ZTokenMut,
    ext_data: &ZCompressedOnlyExtensionInstructionData,
) -> Result<(), ProgramError> {
    let fee: u64 = ext_data.withheld_transfer_fee.into();
    if fee == 0 {
        return Ok(());
    }

    let fee_ext = ctoken.extensions.as_deref_mut().and_then(|exts| {
        exts.iter_mut().find_map(|ext| match ext {
            ZExtensionStructMut::TransferFeeAccount(f) => Some(f),
            _ => None,
        })
    });

    match fee_ext {
        Some(f) => Ok(f.add_withheld_amount(fee)?),
        None => {
            msg!("Decompress: withheld fee but no TransferFeeAccount extension");
            Err(TokenError::DecompressWithheldFeeWithoutExtension.into())
        }
    }
}
