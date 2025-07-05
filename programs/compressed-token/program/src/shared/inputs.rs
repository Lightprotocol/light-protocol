use anchor_lang::solana_program::program_error::ProgramError;
use light_compressed_account::{
    instruction_data::with_readonly::InAccount,
    Pubkey as LightPubkey,
};
use account_compression::StateMerkleTreeAccount;
use anchor_lang::{prelude::*, solana_program::account_info::AccountInfo};
use anchor_compressed_token::{
    process_transfer::{DelegatedTransfer, InputTokenDataWithContext},
    token_data::{AccountState, TokenData},
    ErrorCode,
};
use solana_pubkey::Pubkey;

use super::context::TokenContext;
use crate::constants::TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;

/// Creates a single input compressed account and returns TokenData.
/// Combines the logic from legacy functions into a single composable function.
/// Steps:
/// 1. Determine owner/delegate based on signer and delegate context
/// 2. Check signer permissions for delegate operations
/// 3. Create InAccount with proper discriminator and merkle context
/// 4. Create TokenData with proper state (frozen vs initialized)
/// 5. Compute data hash using TokenContext for caching
/// 6. Return TokenData and lamports for caller use
#[allow(clippy::too_many_arguments)]
pub fn create_input_compressed_account<const IS_FROZEN: bool>(
    input_compressed_account: &mut InAccount,
    context: &mut TokenContext,
    input_token_data: &InputTokenDataWithContext,
    signer: &Pubkey,
    signer_is_delegate: &Option<DelegatedTransfer>,
    remaining_accounts: &[AccountInfo<'_>],
    mint: &Pubkey,
    hashed_mint: &[u8; 32],
) -> std::result::Result<(TokenData, u64), ProgramError> {
    // Determine the owner based on delegate context
    let owner = if input_token_data.delegate_index.is_none() {
        *signer
    } else if let Some(signer_is_delegate) = signer_is_delegate {
        signer_is_delegate.owner
    } else {
        *signer
    };

    // Check signer permissions for delegate operations
    if signer_is_delegate.is_some()
        && input_token_data.delegate_index.is_some()
        && *signer
            != remaining_accounts[input_token_data.delegate_index.unwrap() as usize].key()
    {
        msg!(
            "signer {:?} != delegate in remaining accounts {:?}",
            signer,
            remaining_accounts[input_token_data.delegate_index.unwrap() as usize].key()
        );
        msg!(
            "delegate index {:?}",
            input_token_data.delegate_index.unwrap() as usize
        );
        return Err(ProgramError::Custom(ErrorCode::DelegateSignerCheckFailed as u32));
    }

    // Create InAccount with proper fields
    let lamports = input_token_data.lamports.unwrap_or_default();
    input_compressed_account.lamports = lamports;
    input_compressed_account.discriminator = TOKEN_COMPRESSED_ACCOUNT_DISCRIMINATOR;
    input_compressed_account.merkle_context = input_token_data.merkle_context;
    input_compressed_account.root_index = input_token_data.root_index;
    input_compressed_account.address = None;

    // Create TokenData with proper state
    let state = if IS_FROZEN {
        AccountState::Frozen
    } else {
        AccountState::Initialized
    };

    if input_token_data.tlv.is_some() {
        unimplemented!("Tlv is unimplemented.");
    }

    let token_data = TokenData {
        mint: (*mint).into(),
        owner,
        amount: input_token_data.amount,
        delegate: input_token_data.delegate_index.map(|_| {
            remaining_accounts[input_token_data.delegate_index.unwrap() as usize].key()
        }),
        state,
        tlv: None,
    };

    // Compute data hash using TokenContext for caching
    let hashed_owner = context.get_or_hash_pubkey(&LightPubkey::from(token_data.owner));
    
    let mut amount_bytes = [0u8; 32];
    let discriminator_bytes = &remaining_accounts[input_compressed_account
        .merkle_context
        .merkle_tree_pubkey_index
        as usize]
        .try_borrow_data()?[0..8];
    
    // Handle different discriminator types for amount encoding
    match discriminator_bytes {
        StateMerkleTreeAccount::DISCRIMINATOR => {
            amount_bytes[24..].copy_from_slice(token_data.amount.to_le_bytes().as_slice());
        }
        b"BatchMta" => {
            amount_bytes[24..].copy_from_slice(token_data.amount.to_be_bytes().as_slice());
        }
        b"queueacc" => {
            amount_bytes[24..].copy_from_slice(token_data.amount.to_be_bytes().as_slice());
        }
        _ => {
            msg!(
                "{} is no Merkle tree or output queue account. ",
                remaining_accounts[input_compressed_account
                    .merkle_context
                    .merkle_tree_pubkey_index as usize]
                    .key()
            );
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let hashed_delegate = if let Some(delegate) = token_data.delegate {
        Some(context.get_or_hash_pubkey(&LightPubkey::from(delegate)))
    } else {
        None
    };

    // Use appropriate hash function based on frozen state
    input_compressed_account.data_hash = if !IS_FROZEN {
        TokenData::hash_with_hashed_values(
            hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate.as_ref(),
        )
        .map_err(ProgramError::from)?
    } else {
        TokenData::hash_frozen_with_hashed_values(
            hashed_mint,
            &hashed_owner,
            &amount_bytes,
            &hashed_delegate.as_ref(),
        )
        .map_err(ProgramError::from)?
    };

    Ok((token_data, lamports))
}
