//! Utility functions and default account configurations.

use light_ctoken_interface::instructions::transfer2::MultiInputTokenDataWithContext;
use light_sdk_types::C_TOKEN_PROGRAM_ID;
use solana_account_info::AccountInfo;
use solana_instruction::AccountMeta;
use solana_pubkey::Pubkey;
use spl_pod::bytemuck::pod_from_bytes;
use spl_token_2022::pod::PodAccount;

use crate::{error::CTokenSdkError, AnchorDeserialize, AnchorSerialize};

pub fn get_token_account_balance(token_account_info: &AccountInfo) -> Result<u64, CTokenSdkError> {
    let token_account_data = token_account_info
        .try_borrow_data()
        .map_err(|_| CTokenSdkError::AccountBorrowFailed)?;

    let pod_account = pod_from_bytes::<PodAccount>(&token_account_data)
        .map_err(|_| CTokenSdkError::InvalidAccountData)?;

    Ok(pod_account.amount.into())
}

pub fn is_ctoken_account(account_info: &AccountInfo) -> Result<bool, CTokenSdkError> {
    let ctoken_program_id = Pubkey::from(C_TOKEN_PROGRAM_ID);

    if account_info.owner == &ctoken_program_id {
        return Ok(true);
    }

    let token_22 = spl_token_2022::ID;
    let spl_token = Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    if account_info.owner == &token_22 || account_info.owner == &spl_token {
        return Ok(false);
    }

    Err(CTokenSdkError::CannotDetermineAccountType)
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
use light_compressed_token_types::{
    ACCOUNT_COMPRESSION_AUTHORITY_PDA, ACCOUNT_COMPRESSION_PROGRAM_ID, CPI_AUTHORITY_PDA,
    LIGHT_SYSTEM_PROGRAM_ID, NOOP_PROGRAM_ID, PROGRAM_ID as LIGHT_COMPRESSED_TOKEN_PROGRAM_ID,
};
use light_sdk::constants::REGISTERED_PROGRAM_PDA;

/// Standard pubkeys for compressed token instructions
#[derive(Debug, Copy, Clone)]
pub struct CTokenDefaultAccounts {
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

impl Default for CTokenDefaultAccounts {
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
            compressed_token_program: Pubkey::from(C_TOKEN_PROGRAM_ID),
        }
    }
}
