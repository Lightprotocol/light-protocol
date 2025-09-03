use light_ctoken_types::state::ZExtensionStruct;
use light_zero_copy::traits::ZeroCopyAt;
use solana_account_info::AccountInfo;
use solana_instruction::{AccountMeta, Instruction};
use solana_msg::msg;
use solana_pubkey::Pubkey;

use crate::{
    account2::CTokenAccount2,
    error::TokenSdkError,
    instructions::transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Inputs,
    },
};

/// Struct to hold all the indices needed for CompressAndClose operation
#[derive(Debug)]
pub struct CompressAndCloseIndices {
    pub source_index: u8,
    pub mint_index: u8,
    pub owner_index: u8,
    pub authority_index: u8,
    pub rent_recipient_index: u8,
    pub output_tree_index: u8,
    pub amount: u64,
}

/// Find and validate all required account indices from packed_accounts
fn find_account_indices(
    pubkey_to_index: &std::collections::HashMap<Pubkey, u8>,
    ctoken_account_key: &Pubkey,
    mint_pubkey: impl Into<Pubkey>,
    owner_pubkey: impl Into<Pubkey>,
    authority: impl Into<Pubkey>,
    rent_recipient_pubkey: Pubkey,
    output_tree_pubkey: Pubkey,
    amount: u64,
) -> Result<CompressAndCloseIndices, TokenSdkError> {
    let mint_pubkey = mint_pubkey.into();
    let owner_pubkey = owner_pubkey.into();
    let authority = authority.into();

    let source_index = *pubkey_to_index.get(ctoken_account_key).ok_or_else(|| {
        msg!("Source ctoken account not found in packed_accounts");
        TokenSdkError::InvalidAccountData
    })?;

    let mint_index = *pubkey_to_index.get(&mint_pubkey).ok_or_else(|| {
        msg!("Mint {} not found in packed_accounts", mint_pubkey);
        TokenSdkError::InvalidAccountData
    })?;

    let owner_index = *pubkey_to_index.get(&owner_pubkey).ok_or_else(|| {
        msg!("Owner {} not found in packed_accounts", owner_pubkey);
        TokenSdkError::InvalidAccountData
    })?;

    let authority_index = *pubkey_to_index.get(&authority).ok_or_else(|| {
        msg!("Authority not found in packed_accounts");
        TokenSdkError::InvalidAccountData
    })?;

    let rent_recipient_index = *pubkey_to_index.get(&rent_recipient_pubkey).ok_or_else(|| {
        msg!("Rent recipient not found in packed_accounts");
        TokenSdkError::InvalidAccountData
    })?;

    let output_tree_index = *pubkey_to_index.get(&output_tree_pubkey).ok_or_else(|| {
        msg!("Output tree not found in packed_accounts");
        TokenSdkError::InvalidAccountData
    })?;

    Ok(CompressAndCloseIndices {
        source_index,
        mint_index,
        owner_index,
        authority_index,
        rent_recipient_index,
        output_tree_index,
        amount,
    })
}

/// Compress and close compressed token accounts with pre-computed indices
///
/// # Arguments
/// * `fee_payer` - The fee payer pubkey
/// * `cpi_context_pubkey` - Optional CPI context account for optimized multi-program transactions
/// * `indices` - Slice of pre-computed indices for each account to compress and close
/// * `packed_accounts` - Slice of all accounts that will be used in the instruction (tree accounts)
///
/// # Returns
/// An instruction that compresses and closes all provided token accounts
pub fn compress_and_close_ctoken_accounts_with_indices<'info>(
    fee_payer: Pubkey,
    cpi_context_pubkey: Option<Pubkey>,
    indices: &[CompressAndCloseIndices],
    packed_accounts: &[AccountInfo<'info>],
) -> Result<Instruction, TokenSdkError> {
    if indices.is_empty() {
        return Err(TokenSdkError::InvalidAccountData);
    }

    // Process each set of indices
    let mut token_accounts = Vec::with_capacity(indices.len());

    for idx in indices {
        // Create CTokenAccount2 for CompressAndClose operation
        let mut token_account =
            CTokenAccount2::new_empty(idx.owner_index, idx.mint_index, idx.output_tree_index);

        // Set up compress_and_close with actual indices
        token_account.compress_and_close(
            idx.amount,
            idx.source_index,
            idx.authority_index,
            idx.rent_recipient_index,
        )?;

        token_accounts.push(token_account);
    }

    // Convert packed_accounts to AccountMetas
    let mut packed_account_metas: Vec<AccountMeta> = Vec::with_capacity(packed_accounts.len());
    packed_account_metas.extend(packed_accounts.iter().map(|info| AccountMeta {
        pubkey: *info.key,
        is_signer: info.is_signer,
        is_writable: info.is_writable,
    }));
    let meta_config = if cpi_context_pubkey.is_some() {
        msg!("cpi_context_pubkey is not supported yet");
        unimplemented!()
    } else {
        Transfer2AccountsMetaConfig::new(fee_payer, packed_account_metas)
    };

    // Create the transfer2 instruction with all CompressAndClose operations
    let inputs = Transfer2Inputs {
        meta_config,
        token_accounts,
        //transfer_config: Transfer2Config::default()
        //    .with_cpi_context(cpi_context_pubkey, cpi_context)
        ..Default::default()
    };

    create_transfer2_instruction(inputs)
}

/// Compress and close compressed token accounts
///
/// # Arguments
/// * `fee_payer` - The fee payer pubkey
/// * `with_rent_authority` - If true, use rent authority from compressible token extension
/// * `cpi_context_pubkey` - Optional CPI context account for optimized multi-program transactions
/// * `ctoken_solana_accounts` - Slice of ctoken Solana account infos to compress and close
/// * `packed_accounts` - Slice of all accounts that will be used in the instruction (tree accounts)
///
/// # Returns
/// An instruction that compresses and closes all provided token accounts
pub fn compress_and_close_ctoken_accounts<'info>(
    fee_payer: Pubkey,
    with_rent_authority: bool,
    cpi_context_pubkey: Option<Pubkey>,
    ctoken_solana_accounts: &[&AccountInfo<'info>],
    packed_accounts: &[AccountInfo<'info>],
) -> Result<Instruction, TokenSdkError> {
    if ctoken_solana_accounts.is_empty() {
        return Err(TokenSdkError::InvalidAccountData);
    }

    // TODO: bench replacing it with manual hashmap.
    // Build a mapping of pubkeys to indices in packed_accounts
    let mut pubkey_to_index = std::collections::HashMap::new();
    for (index, account) in packed_accounts.iter().enumerate() {
        pubkey_to_index.insert(*account.key, index as u8);
    }

    // Process each ctoken Solana account and build indices
    let mut indices_vec = Vec::with_capacity(ctoken_solana_accounts.len());
    let mut output_tree_pubkey: Option<Pubkey> = None;

    for ctoken_account_info in ctoken_solana_accounts.iter() {
        let mut rent_recipient_pubkey: Option<Pubkey> = None;
        // Deserialize the ctoken Solana account using light zero copy
        let account_data = ctoken_account_info
            .try_borrow_data()
            .map_err(|_| TokenSdkError::AccountBorrowFailed)?;

        // Deserialize the full CompressedToken including extensions
        let (compressed_token, _) =
            light_ctoken_types::state::CompressedToken::zero_copy_at(&account_data)
                .map_err(|_| TokenSdkError::InvalidAccountData)?;

        // Get the amount (full balance) - convert from zero-copy type
        let amount = u64::from(*compressed_token.amount);

        // Extract pubkeys from the deserialized account
        let mint_pubkey = Pubkey::from(compressed_token.mint.to_bytes());
        let owner_pubkey = Pubkey::from(compressed_token.owner.to_bytes());

        // Check if there's a compressible token extension to get the rent authority
        let authority = if with_rent_authority {
            // Find the compressible token extension
            let mut rent_authority = owner_pubkey;
            if let Some(extensions) = &compressed_token.extensions {
                for extension in extensions {
                    if let ZExtensionStruct::Compressible(extension) = extension {
                        rent_authority = Pubkey::from(extension.rent_authority.to_bytes());
                        break;
                    }
                }
            }
            rent_authority
        } else {
            // If not using rent authority, always use the owner
            owner_pubkey
        };

        // Rent recipient - we need to find this from packed_accounts
        // It should be a writable, non-signer account (often the fee payer or a specific recipient)
        if rent_recipient_pubkey.is_none() {
            // Find the rent recipient - it should be in packed_accounts
            for account in packed_accounts.iter() {
                if account.is_writable
                    && !account.is_signer
                    && *account.key != *ctoken_account_info.key
                {
                    // This could be the rent recipient - we'll use the first writable non-signer that's not a ctoken account
                    // In practice, the caller should ensure the rent recipient is in packed_accounts
                    rent_recipient_pubkey = Some(*account.key);
                    break;
                }
            }
        }

        // Output tree - find the first writable merkle tree account (owned by compression program)
        if output_tree_pubkey.is_none() {
            for account in packed_accounts.iter() {
                if account.is_writable
                    && account.owner.to_bytes() == light_sdk_types::ACCOUNT_COMPRESSION_PROGRAM_ID
                {
                    output_tree_pubkey = Some(*account.key);
                    break;
                }
            }
        }

        // Find indices for all required accounts
        let indices = find_account_indices(
            &pubkey_to_index,
            ctoken_account_info.key,
            mint_pubkey,
            owner_pubkey,
            authority,
            rent_recipient_pubkey.ok_or(TokenSdkError::InvalidAccountData)?,
            output_tree_pubkey.ok_or(TokenSdkError::InvalidAccountData)?,
            amount,
        )?;

        indices_vec.push(indices);
    }
    // Delegate to the with_indices version
    compress_and_close_ctoken_accounts_with_indices(
        fee_payer,
        cpi_context_pubkey,
        &indices_vec,
        packed_accounts,
    )
}
