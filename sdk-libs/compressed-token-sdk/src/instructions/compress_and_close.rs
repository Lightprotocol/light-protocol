use light_ctoken_types::{
    instructions::transfer2::CompressedCpiContext,
    state::{CToken, ZExtensionStruct},
};
use light_program_profiler::profile;
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
#[derive(Debug, Copy, Clone, crate::AnchorSerialize, crate::AnchorDeserialize)]
pub struct CompressAndCloseIndices {
    pub source_index: u8,
    pub mint_index: u8,
    pub owner_index: u8,
    pub authority_index: u8,
    pub rent_sponsor_index: u8,
    pub destination_index: u8,
}

/// Use in the client not in solana program.
///
pub fn pack_for_compress_and_close(
    ctoken_account_pubkey: Pubkey,
    ctoken_account_data: &[u8],
    packed_accounts: &mut PackedAccounts,
    signer_is_compression_authority: bool, // if yes rent authority must be signer
) -> Result<CompressAndCloseIndices, TokenSdkError> {
    let (ctoken_account, _) = CToken::zero_copy_at(ctoken_account_data)?;
    let source_index = packed_accounts.insert_or_get(ctoken_account_pubkey);
    let mint_index = packed_accounts.insert_or_get(Pubkey::from(ctoken_account.mint.to_bytes()));
    let owner_index = packed_accounts.insert_or_get(Pubkey::from(ctoken_account.owner.to_bytes()));

    let (rent_sponsor_index, authority_index, destination_index) =
        if signer_is_compression_authority {
            // When using rent authority from extension, find the rent recipient from extension
            let mut recipient_index = owner_index; // Default to owner if no extension found
            let mut authority_index = owner_index; // Default to owner if no extension found
            if let Some(extensions) = &ctoken_account.extensions {
                for extension in extensions {
                    if let ZExtensionStruct::Compressible(e) = extension {
                        authority_index = packed_accounts.insert_or_get_config(
                            Pubkey::from(e.compression_authority),
                            true,
                            true,
                        );
                        recipient_index =
                            packed_accounts.insert_or_get(Pubkey::from(e.rent_sponsor));

                        break;
                    }
                }
            }
            // When rent authority closes, everything goes to rent recipient
            (recipient_index, authority_index, recipient_index)
        } else {
            // Owner is the authority and needs to sign
            // Check if there's a compressible extension to get the rent_sponsor
            let mut recipient_index = owner_index; // Default to owner if no extension
            if let Some(extensions) = &ctoken_account.extensions {
                for extension in extensions {
                    if let ZExtensionStruct::Compressible(e) = extension {
                        recipient_index =
                            packed_accounts.insert_or_get(Pubkey::from(e.rent_sponsor));

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
                owner_index, // User funds go to owner
            )
        };
    Ok(CompressAndCloseIndices {
        source_index,
        mint_index,
        owner_index,
        authority_index,
        rent_sponsor_index,
        destination_index,
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
    rent_sponsor_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
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

    let rent_sponsor_index = find_index(rent_sponsor_pubkey).ok_or_else(|| {
        msg!("Rent recipient not found in packed_accounts");
        TokenSdkError::InvalidAccountData
    })?;

    let destination_index = find_index(destination_pubkey).ok_or_else(|| {
        msg!("Destination not found in packed_accounts");
        TokenSdkError::InvalidAccountData
    })?;

    Ok(CompressAndCloseIndices {
        source_index,
        mint_index,
        owner_index,
        authority_index,
        rent_sponsor_index,
        destination_index,
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
    rent_sponsor_is_signer: bool,
    cpi_context_pubkey: Option<Pubkey>,
    indices: &[CompressAndCloseIndices],
    packed_accounts: &[AccountInfo<'info>],
) -> Result<Instruction, TokenSdkError> {
    if indices.is_empty() {
        msg!("indices empty");
        return Err(TokenSdkError::InvalidAccountData);
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

        let amount = light_ctoken_types::state::CToken::amount_from_slice(&account_data)?;

        // Create CTokenAccount2 for CompressAndClose operation
        let mut token_account = CTokenAccount2::new_empty(idx.owner_index, idx.mint_index);

        // Set up compress_and_close with actual indices
        token_account.compress_and_close(
            amount,
            idx.source_index,
            idx.authority_index,
            idx.rent_sponsor_index,
            i as u8,               // Pass the index in the output array
            idx.destination_index, // destination for user funds
        )?;
        if rent_sponsor_is_signer {
            packed_account_metas[idx.authority_index as usize].is_signer = true;
        } else {
            packed_account_metas[idx.owner_index as usize].is_signer = true;
        }

        token_accounts.push(token_account);
    }

    let (meta_config, transfer_config) = if let Some(cpi_context) = cpi_context_pubkey {
        let cpi_context_config = CompressedCpiContext {
            set_context: false,
            first_set_context: false,
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
        output_queue: 0, // Output queue is at index 0 in packed_accounts
        ..Default::default()
    };

    create_transfer2_instruction(inputs)
}

/// Compress and close compressed token accounts
///
/// # Arguments
/// * `fee_payer` - The fee payer pubkey
/// * `with_compression_authority` - If true, use rent authority from compressible token extension
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
    with_compression_authority: bool,
    output_queue: AccountInfo<'info>,
    ctoken_solana_accounts: &[&AccountInfo<'info>],
    packed_accounts: &[AccountInfo<'info>],
) -> Result<Instruction, TokenSdkError> {
    if ctoken_solana_accounts.is_empty() {
        msg!("ctoken_solana_accounts empty");
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
        let mut rent_sponsor_pubkey: Option<Pubkey> = None;
        // Deserialize the ctoken Solana account using light zero copy
        let account_data = ctoken_account_info
            .try_borrow_data()
            .map_err(|_| TokenSdkError::AccountBorrowFailed)?;

        // Deserialize the full CToken including extensions
        let (compressed_token, _) = light_ctoken_types::state::CToken::zero_copy_at(&account_data)
            .map_err(|_| TokenSdkError::InvalidAccountData)?;

        // Extract pubkeys from the deserialized account
        let mint_pubkey = Pubkey::from(compressed_token.mint.to_bytes());
        let owner_pubkey = Pubkey::from(compressed_token.owner.to_bytes());

        // Check if there's a compressible token extension to get the rent authority
        let authority = if with_compression_authority {
            // Find the compressible token extension
            let mut compression_authority = owner_pubkey;
            if let Some(extensions) = &compressed_token.extensions {
                for extension in extensions {
                    if let ZExtensionStruct::Compressible(extension) = extension {
                        // Check if compression_authority is set (non-zero)
                        if extension.compression_authority != [0u8; 32] {
                            compression_authority = Pubkey::from(extension.compression_authority);
                        }
                        break;
                    }
                }
            }
            compression_authority
        } else {
            // If not using rent authority, always use the owner
            owner_pubkey
        };

        // Determine rent recipient from extension or use default
        let actual_rent_sponsor = if rent_sponsor_pubkey.is_none() {
            // Check if there's a rent recipient in the compressible extension
            if let Some(extensions) = &compressed_token.extensions {
                for extension in extensions {
                    if let ZExtensionStruct::Compressible(ext) = extension {
                        // Check if rent_sponsor is set (non-zero)
                        if ext.rent_sponsor != [0u8; 32] {
                            rent_sponsor_pubkey = Some(Pubkey::from(ext.rent_sponsor));
                        }
                        break;
                    }
                }
            }

            // If still no rent recipient, find the fee payer (first signer)
            if rent_sponsor_pubkey.is_none() {
                for account in packed_accounts.iter() {
                    if account.is_signer {
                        rent_sponsor_pubkey = Some(*account.key);
                        break;
                    }
                }
            }
            rent_sponsor_pubkey.ok_or(TokenSdkError::InvalidAccountData)?
        } else {
            rent_sponsor_pubkey.unwrap()
        };

        // Determine destination based on authority type
        let destination_pubkey = if with_compression_authority {
            // When rent authority closes, everything goes to rent recipient
            actual_rent_sponsor
        } else {
            // When owner closes, user funds go to owner
            owner_pubkey
        };

        // Find indices for all required accounts
        let indices = find_account_indices(
            find_index,
            ctoken_account_info.key,
            &mint_pubkey,
            &owner_pubkey,
            &authority,
            &actual_rent_sponsor,
            &destination_pubkey,
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
        with_compression_authority,
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
            let mut config = SystemAccountMetaConfig::default();
            config.self_program = self.self_program;
            #[cfg(feature = "cpi-context")]
            {
                config.cpi_context = self.cpi_context;
            }
            #[cfg(not(feature = "cpi-context"))]
            {
                if self.cpi_context.is_some() {
                    msg!("Error: cpi_context is set but 'cpi-context' feature is not enabled");
                    return Err(LightSdkError::ExpectedCpiContext);
                }
            }
            accounts.add_system_accounts_v2(config)?;
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
