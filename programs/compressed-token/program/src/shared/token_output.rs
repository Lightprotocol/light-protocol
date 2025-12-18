// Import the anchor TokenData for hash computation
use anchor_lang::prelude::ProgramError;
use light_compressed_account::{
    instruction_data::data::ZOutputCompressedAccountWithPackedContextMut, Pubkey,
};
use light_ctoken_interface::{
    hash_cache::HashCache,
    instructions::extensions::ZExtensionInstructionData,
    state::{
        CompressedTokenAccountState, ExtensionStructConfig, TokenData, TokenDataConfig,
        TokenDataVersion,
    },
};
use light_hasher::{sha256::Sha256BE, Hasher};
use light_program_profiler::profile;
use light_zero_copy::{num_trait::ZeroCopyNumTrait, ZeroCopyNew};

/// 1. Set token account data
/// 2. Create token account data hash
/// 3. Set output compressed account
#[inline(always)]
#[allow(clippy::too_many_arguments)]
#[profile]
pub fn set_output_compressed_account<'a>(
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut<'_>,
    hash_cache: &mut HashCache,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    amount: impl ZeroCopyNumTrait,
    lamports: Option<impl ZeroCopyNumTrait>,
    mint_pubkey: Pubkey,
    merkle_tree_index: u8,
    version: u8,
    tlv_data: Option<&'a [ZExtensionInstructionData<'a>]>,
    is_frozen: bool,
) -> Result<(), ProgramError> {
    if is_frozen {
        set_output_compressed_account_inner::<true>(
            output_compressed_account,
            hash_cache,
            owner,
            delegate,
            amount,
            lamports,
            mint_pubkey,
            merkle_tree_index,
            version,
            tlv_data,
        )
    } else {
        set_output_compressed_account_inner::<false>(
            output_compressed_account,
            hash_cache,
            owner,
            delegate,
            amount,
            lamports,
            mint_pubkey,
            merkle_tree_index,
            version,
            tlv_data,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn set_output_compressed_account_inner<'a, const IS_FROZEN: bool>(
    output_compressed_account: &mut ZOutputCompressedAccountWithPackedContextMut<'_>,
    hash_cache: &mut HashCache,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    amount: impl ZeroCopyNumTrait,
    lamports: Option<impl ZeroCopyNumTrait>,
    mint_pubkey: Pubkey,
    merkle_tree_index: u8,
    version: u8,
    tlv_data: Option<&'a [ZExtensionInstructionData<'a>]>,
) -> Result<(), ProgramError> {
    // Get compressed account data from CPI struct to temporarily create TokenData
    let compressed_account_data = output_compressed_account
        .compressed_account
        .data
        .as_mut()
        .ok_or(ProgramError::InvalidAccountData)?;

    // Extract config from tlv_data for allocation
    let tlv_config: Option<Vec<ExtensionStructConfig>> = tlv_data.map(|exts| {
        exts.iter()
            .filter_map(|ext| match ext {
                ZExtensionInstructionData::CompressedOnly(_) => {
                    Some(ExtensionStructConfig::CompressedOnly(()))
                }
                _ => None,
            })
            .collect()
    });

    // 1. Set token account data
    {
        // Create token data config based on delegate presence and TLV
        let token_config = TokenDataConfig {
            delegate: (delegate.is_some(), ()),
            tlv: match &tlv_config {
                Some(configs) if !configs.is_empty() => (true, configs.clone()),
                _ => (false, vec![]),
            },
        };
        let (mut token_data, _) =
            TokenData::new_zero_copy(compressed_account_data.data, token_config)
                .map_err(ProgramError::from)?;

        let state = if IS_FROZEN {
            CompressedTokenAccountState::Frozen
        } else {
            CompressedTokenAccountState::Initialized
        };
        token_data.set(mint_pubkey, owner, amount, delegate, state, tlv_data)?;
    }
    let token_version = TokenDataVersion::try_from(version)?;
    // 2. Create TokenData using zero-copy to compute the data hash
    let data_hash = {
        match token_version {
            TokenDataVersion::ShaFlat => Sha256BE::hash(compressed_account_data.data)?,
            _ => {
                let hashed_owner = hash_cache.get_or_hash_pubkey(&owner.into());
                let hashed_mint = hash_cache.get_or_hash_mint(&mint_pubkey.to_bytes())?;

                let amount_bytes = token_version.serialize_amount_bytes(amount.into())?;

                let hashed_delegate = delegate
                    .map(|delegate_pubkey| hash_cache.get_or_hash_pubkey(&delegate_pubkey.into()));

                if !IS_FROZEN {
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
    // 3. Set output compressed account
    let lamports_value = if let Some(value) = lamports {
        value.into()
    } else {
        0u64
    };
    output_compressed_account.set(
        crate::ID.into(),
        lamports_value,
        None, // Token accounts don't have addresses
        merkle_tree_index,
        token_version.discriminator(),
        data_hash,
    )?;

    Ok(())
}
