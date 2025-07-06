use anchor_lang::prelude::{AccountInfo, ProgramError};
use anchor_lang::system_program::ID;
use spl_pod::bytemuck::pod_from_bytes_mut;
use spl_token_2022::pod::PodAccount;

use crate::multi_transfer::{
    accounts::MultiTransferPackedAccounts,
    instruction_data::{ZCompressedTokenInstructionDataMultiTransfer, ZCompression},
};

/// Process native compressions/decompressions with token accounts
pub fn process_token_compression(
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
    packed_accounts: &MultiTransferPackedAccounts,
) -> Result<(), ProgramError> {
    if let Some(compressions) = inputs.compressions.as_ref() {
        for compression in compressions {
            let source_or_recipient = packed_accounts.get_u8(compression.source_or_recipient)?;
            match *source_or_recipient.key {
                ID => {
                    process_native_compressions(compression, source_or_recipient)?;
                }
                _ => return Err(ProgramError::InvalidInstructionData),
            }
        }
    }
    Ok(())
}

/// Process compression/decompression for token accounts using zero-copy PodAccount
fn process_native_compressions(
    compression: &ZCompression,
    token_account_info: &AccountInfo,
) -> Result<(), ProgramError> {
    // Access token account data as mutable bytes
    let mut token_account_data = token_account_info.try_borrow_mut_data()?;

    // Use zero-copy PodAccount to access the token account
    let pod_account = pod_from_bytes_mut::<PodAccount>(&mut token_account_data)
        .map_err(|_| ProgramError::InvalidAccountData)?;

    // Get current balance
    let current_balance: u64 = pod_account.amount.into();

    // Update balance based on compression type
    let new_balance = if compression.is_compress() {
        // Compress: subtract balance (tokens are being compressed)
        current_balance
            .checked_sub(compression.amount.into())
            .ok_or(ProgramError::InsufficientFunds)?
    } else {
        // Decompress: add balance (tokens are being decompressed)
        current_balance
            .checked_add(compression.amount.into())
            .ok_or(ProgramError::ArithmeticOverflow)?
    };

    // Update the balance in the pod account
    pod_account.amount = new_balance.into();

    Ok(())
}
