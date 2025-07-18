use anchor_lang::prelude::ProgramError;
use pinocchio::{account_info::AccountInfo, msg};
use spl_pod::bytemuck::pod_from_bytes_mut;
use spl_token_2022::pod::PodAccount;

use crate::{multi_transfer::accounts::MultiTransferPackedAccounts, LIGHT_CPI_SIGNER};
use light_ctoken_types::instructions::multi_transfer::{
    ZCompressedTokenInstructionDataMultiTransfer, ZCompression,
};
const ID: &[u8; 32] = &LIGHT_CPI_SIGNER.program_id;
/// Process native compressions/decompressions with token accounts
pub fn process_token_compression(
    inputs: &ZCompressedTokenInstructionDataMultiTransfer,
    packed_accounts: &MultiTransferPackedAccounts,
) -> Result<(), ProgramError> {
    if let Some(compressions) = inputs.compressions.as_ref() {
        for compression in compressions {
            let source_or_recipient = packed_accounts.get_u8(compression.source_or_recipient)?;
            use anchor_lang::solana_program::log::msg;
            msg!(
                "source_or_recipient: {:?}",
                solana_pubkey::Pubkey::new_from_array(*source_or_recipient.key())
            );
            msg!(
                "source_or_recipient: {:?}",
                solana_pubkey::Pubkey::new_from_array(unsafe { *source_or_recipient.owner() })
            );

            match unsafe { source_or_recipient.owner() } {
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
    msg!("process_native_compressions");

    // Access token account data as mutable bytes
    let mut token_account_data = token_account_info
        .try_borrow_mut_data()
        .map_err(|_| ProgramError::AccountBorrowFailed)?;
    msg!("pre pod");
    // Use zero-copy PodAccount to access the token account
    let pod_account = pod_from_bytes_mut::<PodAccount>(&mut token_account_data)
        .map_err(|e| ProgramError::Custom(u64::from(e) as u32))?;
    msg!(format!("pod_account {:?}", pod_account).as_str());

    // Get current balance
    let current_balance: u64 = pod_account.amount.into();

    // Update balance based on compression type
    let new_balance = compression.new_balance_solana_account(current_balance)?;

    // Update the balance in the pod account
    pod_account.amount = new_balance.into();

    Ok(())
}
