use light_compressed_account::{
    instruction_data::cpi_context::CompressedCpiContext, pubkey::AsPubkey,
};
use light_ctoken_types::state::{CompressedToken, ZExtensionStruct};
use light_profiler::profile;
use light_sdk::{
    error::LightSdkError,
    instruction::{AccountMetasVec, PackedAccounts, SystemAccountMetaConfig},
};
use light_zero_copy::traits::ZeroCopyAt;
use solana_account_info::AccountInfo;
use solana_instruction::{AccountMeta, Instruction};
use solana_msg::msg;
use solana_pubkey::Pubkey;

use crate::{
    account2::CTokenAccount2,
    error::TokenSdkError,
    instructions::{
        transfer2::{
            account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction,
            Transfer2Config, Transfer2Inputs,
        },
        CTokenDefaultAccounts,
    },
};

/// Struct to hold all the indices needed for CompressAndClose operation
#[derive(Debug, crate::AnchorSerialize, crate::AnchorDeserialize)]
pub struct CompressAndCloseIndices {
    pub source_index: u8,
    pub mint_index: u8,
    pub owner_index: u8,
    pub authority_index: u8,
    pub rent_recipient_index: u8,
    pub output_tree_index: u8,
}

/// Use in the client not in solana program.
///
pub fn pack_for_compress_and_close(
    ctoken_account_pubkey: Pubkey,
    ctoken_account_data: &[u8],
    output_queue: Pubkey,
    packed_accounts: &mut PackedAccounts,
    signer_is_rent_authority: bool, // if yes rent authority must be signer
) -> Result<CompressAndCloseIndices, TokenSdkError> {
    // Add output queue first so it's at index 0
    let output_tree_index = packed_accounts.insert_or_get(output_queue);
    let (ctoken_account, _) = CompressedToken::zero_copy_at(ctoken_account_data)?;
    let source_index = packed_accounts.insert_or_get(ctoken_account_pubkey);
    let mint_index = packed_accounts.insert_or_get(Pubkey::from(ctoken_account.mint.to_bytes()));
    let owner_index = packed_accounts.insert_or_get(Pubkey::from(ctoken_account.owner.to_bytes()));

    let (rent_recipient_index, authority_index) = if signer_is_rent_authority {
        // When using rent authority from extension, find the rent recipient from extension
        let mut recipient_index = owner_index; // Default to owner if no extension found
        let mut authority_index = owner_index; // Default to owner if no extension found
        if let Some(extensions) = &ctoken_account.extensions {
            for extension in extensions {
                if let ZExtensionStruct::Compressible(e) = extension {
                    let rent_authority =
                        e.rent_authority.ok_or(TokenSdkError::RentAuthorityIsNone)?;
                    authority_index = packed_accounts.insert_or_get_config(
                        Pubkey::from(*rent_authority),
                        true,
                        true,
                    );
                    if let Some(rent_recipient) = e.rent_recipient.as_deref() {
                        recipient_index =
                            packed_accounts.insert_or_get(Pubkey::from(*rent_recipient));
                    }
                    break;
                }
            }
        }
        (recipient_index, authority_index)
    } else {
        // Owner is the authority and needs to sign
        // Check if there's a compressible extension to get the rent_recipient
        let mut recipient_index = owner_index; // Default to owner if no extension
        if let Some(extensions) = &ctoken_account.extensions {
            for extension in extensions {
                if let ZExtensionStruct::Compressible(e) = extension {
                    if let Some(rent_recipient) = e.rent_recipient.as_deref() {
                        recipient_index =
                            packed_accounts.insert_or_get(Pubkey::from(*rent_recipient));
                    }
                    break;
                }
            }
        }
        (
            recipient_index,
            packed_accounts.insert_or_get_config(
                Pubkey::from(ctoken_account.owner.to_bytes()),
                true,
                false,
            ),
        )
    };
    Ok(CompressAndCloseIndices {
        source_index,
        mint_index,
        owner_index,
        authority_index,
        rent_recipient_index,
        output_tree_index,
    })
}

/// Find and validate all required account indices from packed_accounts
#[inline(always)]
#[profile]
fn find_account_indices(
    find_index: impl Fn(&Pubkey) -> Option<u8>,
    ctoken_account_key: &Pubkey,
    mint_pubkey: &Pubkey,
    owner_pubkey: &Pubkey,
    authority: &Pubkey,
    rent_recipient_pubkey: &Pubkey,
    // output_tree_pubkey: &Pubkey,
) -> Result<CompressAndCloseIndices, TokenSdkError> {
    let source_index = find_index(ctoken_account_key).ok_or_else(|| {
        msg!("Source ctoken account not found in packed_accounts");
        TokenSdkError::InvalidAccountData
    })?;

    let mint_index = find_index(mint_pubkey).ok_or_else(|| {
        msg!("Mint {} not found in packed_accounts", mint_pubkey);
        TokenSdkError::InvalidAccountData
    })?;

    let owner_index = find_index(owner_pubkey).ok_or_else(|| {
        msg!("Owner {} not found in packed_accounts", owner_pubkey);
        TokenSdkError::InvalidAccountData
    })?;

    let authority_index = find_index(authority).ok_or_else(|| {
        msg!("Authority not found in packed_accounts");
        TokenSdkError::InvalidAccountData
    })?;

    let rent_recipient_index = find_index(rent_recipient_pubkey).ok_or_else(|| {
        msg!("Rent recipient not found in packed_accounts");
        TokenSdkError::InvalidAccountData
    })?;

    Ok(CompressAndCloseIndices {
        source_index,
        mint_index,
        owner_index,
        authority_index,
        rent_recipient_index,
        output_tree_index: 0,
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
#[profile]
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

    for (i, idx) in indices.iter().enumerate() {
        // Get the amount from the source token account
        let source_account = packed_accounts
            .get(idx.source_index as usize)
            .ok_or(TokenSdkError::InvalidAccountData)?;

        let account_data = source_account
            .try_borrow_data()
            .map_err(|_| TokenSdkError::AccountBorrowFailed)?;

        let amount = light_ctoken_types::state::CompressedToken::amount_from_slice(&account_data)?;

        // Create CTokenAccount2 for CompressAndClose operation
        let mut token_account =
            CTokenAccount2::new_empty(idx.owner_index, idx.mint_index, idx.output_tree_index);

        // Set up compress_and_close with actual indices
        token_account.compress_and_close(
            amount,
            idx.source_index,
            idx.authority_index,
            idx.rent_recipient_index,
            i as u8, // Pass the index in the output array
        )?;

        token_accounts.push(token_account);
    }

    // Convert packed_accounts to AccountMetas using ArrayVec to avoid heap allocation
    let mut packed_account_metas = arrayvec::ArrayVec::<AccountMeta, 32>::new();
    for info in packed_accounts.iter() {
        packed_account_metas.push(AccountMeta {
            pubkey: *info.key,
            is_signer: info.is_signer,
            is_writable: info.is_writable,
        });
    }
    let (meta_config, transfer_config) = if let Some(cpi_context) = cpi_context_pubkey {
        let cpi_context_config = CompressedCpiContext {
            set_context: false,
            first_set_context: false,
            cpi_context_account_index: 0, // unused
        };

        (
            Transfer2AccountsMetaConfig {
                fee_payer: Some(fee_payer),
                cpi_context: Some(cpi_context),
                decompressed_accounts_only: false,
                sol_pool_pda: None,
                sol_decompression_recipient: None,
                with_sol_pool: false,
                packed_accounts: Some(packed_account_metas.to_vec()),
            },
            Transfer2Config::default().with_cpi_context(cpi_context_config),
        )
    } else {
        (
            Transfer2AccountsMetaConfig::new(fee_payer, packed_account_metas.to_vec()),
            Transfer2Config::default(),
        )
    };

    // Create the transfer2 instruction with all CompressAndClose operations
    let inputs = Transfer2Inputs {
        meta_config,
        token_accounts,
        transfer_config,
        ..Default::default()
    };

    create_transfer2_instruction(inputs)
}

/// Compress and close compressed token accounts
///
/// # Arguments
/// * `fee_payer` - The fee payer pubkey
/// * `with_rent_authority` - If true, use rent authority from compressible token extension
/// * `output_queue_pubkey` - The output queue pubkey where compressed accounts will be stored
/// * `cpi_context_pubkey` - Optional CPI context account for optimized multi-program transactions
/// * `ctoken_solana_accounts` - Slice of ctoken Solana account infos to compress and close
/// * `packed_accounts` - Slice of all accounts that will be used in the instruction (tree accounts)
///
/// # Returns
/// An instruction that compresses and closes all provided token accounts
#[profile]
pub fn compress_and_close_ctoken_accounts<'info>(
    fee_payer: Pubkey,
    with_rent_authority: bool,
    output_queue: AccountInfo<'info>,
    ctoken_solana_accounts: &[&AccountInfo<'info>],
    packed_accounts: &[AccountInfo<'info>],
) -> Result<Instruction, TokenSdkError> {
    if ctoken_solana_accounts.is_empty() {
        return Err(TokenSdkError::InvalidAccountData);
    }

    // Helper function to find index of a pubkey in packed_accounts using linear search
    // More efficient than HashMap for small arrays in Solana programs
    // Note: We add 1 to account for output_queue being inserted at index 0 later
    let find_index = |pubkey: &Pubkey| -> Option<u8> {
        packed_accounts
            .iter()
            .position(|account| account.key == pubkey)
            .map(|idx| (idx + 1) as u8) // Add 1 because output_queue will be at index 0
    };

    // Process each ctoken Solana account and build indices
    let mut indices_vec = Vec::with_capacity(ctoken_solana_accounts.len());

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
                        rent_authority =
                            Pubkey::from(extension.rent_authority.unwrap().to_pubkey_bytes());
                        break;
                    }
                }
            }
            rent_authority
        } else {
            // If not using rent authority, always use the owner
            owner_pubkey
        };

        // Determine rent recipient from extension or use default
        let actual_rent_recipient = if rent_recipient_pubkey.is_none() {
            // Check if there's a rent recipient in the compressible extension
            if let Some(extensions) = &compressed_token.extensions {
                for extension in extensions {
                    if let ZExtensionStruct::Compressible(ext) = extension {
                        rent_recipient_pubkey =
                            Some(Pubkey::from(ext.rent_recipient.unwrap().to_pubkey_bytes()));
                        break;
                    }
                }
            }

            // If still no rent recipient, find the fee payer (first signer)
            if rent_recipient_pubkey.is_none() {
                for account in packed_accounts.iter() {
                    if account.is_signer {
                        rent_recipient_pubkey = Some(*account.key);
                        break;
                    }
                }
            }
            rent_recipient_pubkey.ok_or(TokenSdkError::InvalidAccountData)?
        } else {
            rent_recipient_pubkey.unwrap()
        };

        // Find indices for all required accounts
        let indices = find_account_indices(
            find_index,
            ctoken_account_info.key,
            &mint_pubkey,
            &owner_pubkey,
            &authority,
            &actual_rent_recipient,
            // &output_queue_pubkey,
        )?;
        indices_vec.push(indices);
    }
    let mut packed_accounts_vec = Vec::with_capacity(packed_accounts.len() + 1);
    packed_accounts_vec.push(output_queue);
    packed_accounts_vec.extend_from_slice(packed_accounts);

    // Delegate to the with_indices version
    compress_and_close_ctoken_accounts_with_indices(
        fee_payer,
        None,
        &indices_vec,
        packed_accounts_vec.as_slice(),
    )
}

pub struct CompressAndCloseAccounts {
    pub compressed_token_program: Pubkey,
    pub cpi_authority_pda: Pubkey,
    pub cpi_context: Option<Pubkey>,
    pub self_program: Option<Pubkey>,
}

impl Default for CompressAndCloseAccounts {
    fn default() -> Self {
        Self {
            compressed_token_program: CTokenDefaultAccounts::default().compressed_token_program,
            cpi_authority_pda: CTokenDefaultAccounts::default().cpi_authority_pda,
            cpi_context: None,
            self_program: None,
        }
    }
}

impl CompressAndCloseAccounts {
    pub fn new_with_cpi_context(cpi_context: Option<Pubkey>, self_program: Option<Pubkey>) -> Self {
        Self {
            compressed_token_program: CTokenDefaultAccounts::default().compressed_token_program,
            cpi_authority_pda: CTokenDefaultAccounts::default().cpi_authority_pda,
            cpi_context,
            self_program,
        }
    }
}

impl AccountMetasVec for CompressAndCloseAccounts {
    /// Adds:
    /// 1. system accounts if not set
    /// 2. compressed token program and ctoken cpi authority pda to pre accounts
    fn get_account_metas_vec(&self, accounts: &mut PackedAccounts) -> Result<(), LightSdkError> {
        if !accounts.system_accounts_set() {
            let config = SystemAccountMetaConfig {
                self_program: self.self_program,
                cpi_context: self.cpi_context,
                ..Default::default()
            };
            accounts.add_system_accounts_small(config)?;
        }
        // Add both accounts in one operation for better performance
        accounts.pre_accounts.extend_from_slice(&[
            AccountMeta {
                pubkey: self.compressed_token_program,
                is_signer: false,
                is_writable: false,
            },
            AccountMeta {
                pubkey: self.cpi_authority_pda,
                is_signer: false,
                is_writable: false,
            },
        ]);
        Ok(())
    }
}
