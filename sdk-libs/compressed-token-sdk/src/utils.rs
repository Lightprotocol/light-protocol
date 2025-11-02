#[cfg(feature = "anchor")]
use anchor_lang::prelude::InterfaceAccount;
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
// #[cfg(feature = "anchor")]
// use anchor_spl::token_interface;
use light_ctoken_types::instructions::transfer2::MultiInputTokenDataWithContext;
use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;

use crate::error::TokenSdkError;

/// Get token account balance from account info
pub fn get_token_account_balance(token_account_info: &AccountInfo) -> Result<u64, TokenSdkError> {
    let token_account_data = token_account_info
        .try_borrow_data()
        .map_err(|_| TokenSdkError::AccountBorrowFailed)?;

    // Use zero-copy PodAccount to access the token account
    let pod_account = pod_from_bytes::<PodAccount>(&token_account_data)
        .map_err(|_| TokenSdkError::InvalidAccountData)?;

    Ok(pod_account.amount.into())
}

/// Evaluate if an account is a CToken account
///
/// Returns true if owned by CToken program, false if owned by SPL Token or
/// Token-2022.
pub fn is_ctoken_account(account_info: &AccountInfo) -> Result<bool, TokenSdkError> {
    let ctoken_program_id = Pubkey::from(C_TOKEN_PROGRAM_ID);

    if account_info.owner == &ctoken_program_id {
        return Ok(true);
    }

    let token_22 = spl_token_2022::ID;
    let spl_token = Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    if account_info.owner == &token_22 || account_info.owner == &spl_token {
        return Ok(false);
    }

    // Must be one of the three.
    Err(TokenSdkError::CannotDetermineAccountType)
}

/// Same as SPL-token discriminator
pub const CLOSE_TOKEN_ACCOUNT_DISCRIMINATOR: u8 = 9;

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct PackedCompressedTokenDataWithContext {
    pub mint: u8,
    pub source_or_recipient_token_account: u8,
    pub multi_input_token_data_with_context: MultiInputTokenDataWithContext,
}

pub fn account_meta_from_account_info(account_info: &AccountInfo) -> AccountMeta {
    AccountMeta {
        pubkey: *account_info.key,
        is_signer: account_info.is_signer,
        is_writable: account_info.is_writable,
    }
}

// /// Structure to hold token account data for batch compression
// #[cfg(feature = "anchor")]
// #[derive(Debug, Clone)]
// pub struct TokenAccountToCompress<'info> {
//     pub token_account: InterfaceAccount<'info, token_interface::TokenAccount>,
//     pub signer_seeds: Vec<Vec<u8>>,
// }

#[derive(Debug, Clone)]
pub struct AccountInfoToCompress<'info> {
    pub account_info: AccountInfo<'info>,
    pub signer_seeds: Vec<Vec<u8>>,
}

fn add_or_get_index<T: PartialEq>(vec: &mut Vec<T>, item: T) -> u8 {
    if let Some(idx) = vec.iter().position(|x| x == &item) {
        idx as u8
    } else {
        vec.push(item);
        (vec.len() - 1) as u8
    }
}
