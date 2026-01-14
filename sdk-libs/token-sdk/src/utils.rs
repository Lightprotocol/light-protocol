//! Utility functions and default account configurations.

use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token_interface::{
    instructions::transfer2::MultiInputTokenDataWithContext, state::Token,
};
use solana_account_info::AccountInfo;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;

use crate::{error::TokenSdkError, AnchorDeserialize, AnchorSerialize};

pub fn get_token_account_balance(token_account_info: &AccountInfo) -> Result<u64, TokenSdkError> {
    let data = token_account_info
        .try_borrow_data()
        .map_err(|_| TokenSdkError::AccountBorrowFailed)?;
    Token::amount_from_slice(&data).map_err(|_| TokenSdkError::InvalidAccountData)
}

pub fn is_token_account(account_info: &AccountInfo) -> Result<bool, TokenSdkError> {
    let light_token_program_id = Pubkey::from(LIGHT_TOKEN_PROGRAM_ID);

    if account_info.owner == &light_token_program_id {
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
use light_sdk::constants::REGISTERED_PROGRAM_PDA;
use light_token_types::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA,
    LIGHT_SYSTEM_PROGRAM_ID, NOOP_PROGRAM_ID, PROGRAM_ID as LIGHT_COMPRESSED_TOKEN_PROGRAM_ID,
};

/// Standard pubkeys for compressed token instructions
#[derive(Debug, Copy, Clone)]
pub struct TokenDefaultAccounts {
    pub light_system_program: Pubkey,
    pub registered_program_pda: Pubkey,
    pub noop_program: Pubkey,
    pub account_compression_authority: Pubkey,
    pub account_compression_program: Pubkey,
    pub self_program: Pubkey,
    pub cpi_authority_pda: Pubkey,
    pub system_program: Pubkey,
    pub compressed_token_program: Pubkey,
}

impl Default for TokenDefaultAccounts {
    fn default() -> Self {
        Self {
            light_system_program: Pubkey::from(LIGHT_SYSTEM_PROGRAM_ID),
            registered_program_pda: Pubkey::from(REGISTERED_PROGRAM_PDA),
            noop_program: Pubkey::from(NOOP_PROGRAM_ID),
            account_compression_authority: Pubkey::from(ACCOUNT_COMPRESSION_AUTHORITY_PDA),
            account_compression_program: Pubkey::from(ACCOUNT_COMPRESSION_PROGRAM_ID),
            self_program: Pubkey::from(LIGHT_COMPRESSED_TOKEN_PROGRAM_ID),
            cpi_authority_pda: Pubkey::from(CPI_AUTHORITY_PDA),
            system_program: Pubkey::default(),
            compressed_token_program: Pubkey::from(LIGHT_TOKEN_PROGRAM_ID),
        }
    }
}
