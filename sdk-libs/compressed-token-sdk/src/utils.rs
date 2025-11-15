use light_ctoken_types::instructions::transfer2::MultiInputTokenDataWithContext;
use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;

use crate::{error::TokenSdkError, AnchorDeserialize, AnchorSerialize};

pub fn get_token_account_balance(token_account_info: &AccountInfo) -> Result<u64, TokenSdkError> {
    let token_account_data = token_account_info
        .try_borrow_data()
        .map_err(|_| TokenSdkError::AccountBorrowFailed)?;

    let pod_account = pod_from_bytes::<PodAccount>(&token_account_data)
        .map_err(|_| TokenSdkError::InvalidAccountData)?;

    Ok(pod_account.amount.into())
}

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

    Err(TokenSdkError::CannotDetermineAccountType)
}

pub const CLOSE_TOKEN_ACCOUNT_DISCRIMINATOR: u8 = 9;

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
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

#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct AccountInfoToCompress<'info> {
    pub account_info: AccountInfo<'info>,
    pub signer_seeds: Vec<Vec<u8>>,
}
