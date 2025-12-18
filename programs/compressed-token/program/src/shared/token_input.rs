use std::panic::Location;

use anchor_compressed_token::TokenData;
use anchor_lang::solana_program::program_error::ProgramError;
use light_account_checks::AccountError;
use light_compressed_account::instruction_data::with_readonly::ZInAccountMut;
use light_ctoken_interface::{
    hash_cache::HashCache,
    instructions::{
        extensions::ZExtensionInstructionData, transfer2::ZMultiInputTokenDataWithContext,
    },
    state::{
        CompressedOnlyExtension, CompressedTokenAccountState, ExtensionStruct, TokenDataVersion,
    },
};
use pinocchio::account_info::AccountInfo;

use crate::{
    shared::owner_validation::verify_owner_or_delegate_signer,
    transfer2::check_extensions::MintExtensionCache,
};

/// Creates an input compressed account using zero-copy patterns and index-based account lookup.
///
/// Validates signer authorization (owner, delegate, or permanent delegate), populates the
/// zero-copy account structure, and computes the appropriate token data hash based on frozen state.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
pub fn set_input_compressed_account<'a>(
    input_compressed_account: &mut ZInAccountMut,
    hash_cache: &mut HashCache,
    input_token_data: &ZMultiInputTokenDataWithContext,
    packed_accounts: &[AccountInfo],
    all_accounts: &[AccountInfo],
    lamports: u64,
    tlv_data: Option<&'a [ZExtensionInstructionData<'a>]>,
    mint_cache: &MintExtensionCache,
    is_frozen: bool,
) -> std::result::Result<(), ProgramError> {
    // Get owner from packed accounts using the owner index
    let owner_account = packed_accounts
        .get(input_token_data.owner as usize)
        .ok_or_else(|| {
            print_on_error_pubkey(input_token_data.owner, "owner", Location::caller());
            ProgramError::Custom(AccountError::NotEnoughAccountKeys.into())
        })?;

    // Verify signer authorization using shared function
    let delegate_account = if input_token_data.has_delegate() {
        Some(
            packed_accounts
                .get(input_token_data.delegate as usize)
                .ok_or_else(|| {
                    print_on_error_pubkey(
                        input_token_data.delegate,
                        "delegate",
                        Location::caller(),
                    );
                    ProgramError::Custom(AccountError::NotEnoughAccountKeys.into())
                })?,
        )
    } else {
        None
    };

    // Get mint account early for hashing
    let mint_account = &packed_accounts
        .get(input_token_data.mint as usize)
        .ok_or_else(|| {
            print_on_error_pubkey(input_token_data.mint, "mint", Location::caller());
            ProgramError::Custom(AccountError::NotEnoughAccountKeys.into())
        })?;

    // Lookup permanent delegate for mint account.
    let permanent_delegate = mint_cache
        .get_by_key(&input_token_data.mint)
        .and_then(|c| c.permanent_delegate.as_ref());

    verify_owner_or_delegate_signer(
        owner_account,
        delegate_account,
        permanent_delegate,
        all_accounts,
    )?;
    let token_version = TokenDataVersion::try_from(input_token_data.version)?;

    let data_hash = {
        match token_version {
            TokenDataVersion::ShaFlat => {
                let state = if is_frozen {
                    CompressedTokenAccountState::Frozen as u8
                } else {
                    CompressedTokenAccountState::Initialized as u8
                };
                // Convert instruction TLV data to state TLV
                let tlv: Option<Vec<ExtensionStruct>> = tlv_data.map(|exts| {
                    exts.iter()
                        .filter_map(|ext| match ext {
                            ZExtensionInstructionData::CompressedOnly(data) => {
                                Some(ExtensionStruct::CompressedOnly(CompressedOnlyExtension {
                                    delegated_amount: data.delegated_amount.into(),
                                    withheld_transfer_fee: data.withheld_transfer_fee.into(),
                                }))
                            }
                            _ => None,
                        })
                        .collect()
                });
                let token_data = TokenData {
                    mint: mint_account.key().into(),
                    owner: owner_account.key().into(),
                    amount: input_token_data.amount.into(),
                    delegate: delegate_account.map(|x| (*x.key()).into()),
                    state,
                    tlv,
                };
                token_data.hash_sha_flat()?
            }
            _ => {
                let hashed_owner = hash_cache.get_or_hash_pubkey(owner_account.key());
                // Get mint hash from hash_cache
                let hashed_mint = hash_cache.get_or_hash_mint(mint_account.key())?;
                let amount_bytes =
                    token_version.serialize_amount_bytes(input_token_data.amount.into())?;

                let hashed_delegate =
                    delegate_account.map(|delegate| hash_cache.get_or_hash_pubkey(delegate.key()));

                if !is_frozen {
                    TokenData::hash_with_hashed_values(
                        &hashed_mint,
                        &hashed_owner,
                        &amount_bytes,
                        &hashed_delegate.as_ref(),
                    )
                } else {
                    TokenData::hash_frozen_with_hashed_values(
                        &hashed_mint,
                        &hashed_owner,
                        &amount_bytes,
                        &hashed_delegate.as_ref(),
                    )
                }
            }?,
        }
    };

    input_compressed_account.set_z(
        token_version.discriminator(),
        data_hash,
        &input_token_data.merkle_context,
        *input_token_data.root_index,
        lamports,
        None,
    )?;
    Ok(())
}

#[cold]
fn print_on_error_pubkey(index: u8, account_name: &str, location: &Location) {
    anchor_lang::prelude::msg!(
        "ERROR: out of bounds. for account '{}' at index {}  {}:{}:{}",
        account_name,
        index,
        location.file(),
        location.line(),
        location.column()
    );
}
