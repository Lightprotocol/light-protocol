use light_compressed_account::compressed_account::PackedMerkleContext;
use light_ctoken_types::instructions::transfer2::{
    CompressedCpiContext, MultiInputTokenDataWithContext,
};
use light_program_profiler::profile;
use light_sdk::{
    error::LightSdkError,
    instruction::{AccountMetasVec, PackedAccounts, PackedStateTreeInfo, SystemAccountMetaConfig},
    token::TokenData,
};
use solana_account_info::AccountInfo;
use solana_instruction::{AccountMeta, Instruction};
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
    ValidityProof,
};

/// Struct to hold all the data needed for DecompressFull operation
/// Contains the complete compressed account data and destination index
#[derive(Debug, Clone, crate::AnchorSerialize, crate::AnchorDeserialize)]
pub struct DecompressFullIndices {
    pub source: MultiInputTokenDataWithContext, // Complete compressed account data with merkle context
    pub destination_index: u8,                  // Destination ctoken Solana account (must exist)
}

/// Decompress full balance from compressed token accounts with pre-computed indices
///
/// # Arguments
/// * `fee_payer` - The fee payer pubkey
/// * `validity_proof` - Validity proof for the compressed accounts (zkp or index)
/// * `cpi_context_pubkey` - Optional CPI context account for optimized multi-program transactions
/// * `indices` - Slice of source/destination pairs for decompress operations
/// * `packed_accounts` - Slice of all accounts that will be used in the instruction
///
/// # Returns
/// An instruction that decompresses the full balance of all provided token accounts
#[profile]
pub fn decompress_full_ctoken_accounts_with_indices<'info>(
    fee_payer: Pubkey,
    validity_proof: ValidityProof,
    cpi_context_pubkey: Option<Pubkey>,
    indices: &[DecompressFullIndices],
    packed_accounts: &[AccountInfo<'info>],
) -> Result<Instruction, TokenSdkError> {
    if indices.is_empty() {
        return Err(TokenSdkError::InvalidAccountData);
    }

    // Process each set of indices
    let mut token_accounts = Vec::with_capacity(indices.len());

    for idx in indices.iter() {
        // Create CTokenAccount2 with the source data
        // For decompress_full, we don't have an output tree since everything goes to the destination
        let mut token_account = CTokenAccount2::new(vec![idx.source])?;

        // Set up decompress_full - decompress entire balance to destination ctoken account
        token_account.decompress_ctoken(idx.source.amount, idx.destination_index)?;
        token_accounts.push(token_account);
    }

    // Convert packed_accounts to AccountMetas
    let mut packed_account_metas = Vec::with_capacity(packed_accounts.len());
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
        };

        (
            Transfer2AccountsMetaConfig {
                fee_payer: Some(fee_payer),
                cpi_context: Some(cpi_context),
                decompressed_accounts_only: false,
                sol_pool_pda: None,
                sol_decompression_recipient: None,
                with_sol_pool: false,
                packed_accounts: Some(packed_account_metas),
            },
            Transfer2Config::default()
                .filter_zero_amount_outputs()
                .with_cpi_context(cpi_context_config),
        )
    } else {
        (
            Transfer2AccountsMetaConfig::new(fee_payer, packed_account_metas),
            Transfer2Config::default().filter_zero_amount_outputs(),
        )
    };

    // Create the transfer2 instruction with all decompress operations
    let inputs = Transfer2Inputs {
        meta_config,
        token_accounts,
        transfer_config,
        validity_proof,
        ..Default::default()
    };

    create_transfer2_instruction(inputs)
}

/// Helper function to pack compressed token accounts into DecompressFullIndices
/// Used in tests to build indices for multiple compressed accounts to decompress
///
/// # Arguments
/// * `token_data` - Slice of TokenData from compressed accounts
/// * `tree_infos` - Packed tree info for each compressed account
/// * `destination_indices` - Destination account indices for each decompression
/// * `packed_accounts` - PackedAccounts that will be used to insert/get indices
///
/// # Returns
/// Vec of DecompressFullIndices ready to use with decompress_full_ctoken_accounts_with_indices
#[profile]
pub fn pack_for_decompress_full(
    token: &TokenData,
    tree_info: &PackedStateTreeInfo,
    destination: Pubkey,
    packed_accounts: &mut PackedAccounts,
) -> DecompressFullIndices {
    let source = MultiInputTokenDataWithContext {
        owner: packed_accounts.insert_or_get_config(token.owner, true, false),
        amount: token.amount,
        has_delegate: token.delegate.is_some(),
        delegate: token
            .delegate
            .map(|d| packed_accounts.insert_or_get(d))
            .unwrap_or(0),
        mint: packed_accounts.insert_or_get(token.mint),
        version: 2,
        merkle_context: PackedMerkleContext {
            merkle_tree_pubkey_index: tree_info.merkle_tree_pubkey_index,
            queue_pubkey_index: tree_info.queue_pubkey_index,
            prove_by_index: tree_info.prove_by_index,
            leaf_index: tree_info.leaf_index,
        },
        root_index: tree_info.root_index,
    };

    DecompressFullIndices {
        source,
        destination_index: packed_accounts.insert_or_get(destination),
    }
}

pub struct DecompressFullAccounts {
    pub compressed_token_program: Pubkey,
    pub cpi_authority_pda: Pubkey,
    pub cpi_context: Option<Pubkey>,
    pub self_program: Option<Pubkey>,
}

impl DecompressFullAccounts {
    pub fn new(cpi_context: Option<Pubkey>) -> Self {
        Self {
            compressed_token_program: CTokenDefaultAccounts::default().compressed_token_program,
            cpi_authority_pda: CTokenDefaultAccounts::default().cpi_authority_pda,
            cpi_context,
            self_program: None,
        }
    }
    pub fn new_with_cpi_context(cpi_context: Option<Pubkey>, self_program: Option<Pubkey>) -> Self {
        Self {
            compressed_token_program: CTokenDefaultAccounts::default().compressed_token_program,
            cpi_authority_pda: CTokenDefaultAccounts::default().cpi_authority_pda,
            cpi_context,
            self_program,
        }
    }
}

impl AccountMetasVec for DecompressFullAccounts {
    /// Adds:
    /// 1. system accounts if not set
    /// 2. compressed token program and ctoken cpi authority pda to pre accounts
    fn get_account_metas_vec(&self, accounts: &mut PackedAccounts) -> Result<(), LightSdkError> {
        if !accounts.system_accounts_set() {
            #[cfg(feature = "cpi-context")]
            let config = {
                let mut config = SystemAccountMetaConfig::default();
                config.self_program = self.self_program;
                config.cpi_context = self.cpi_context;
                config
            };
            #[cfg(not(feature = "cpi-context"))]
            let config = {
                let mut config = SystemAccountMetaConfig::default();
                config.self_program = self.self_program;
                config
            };

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
