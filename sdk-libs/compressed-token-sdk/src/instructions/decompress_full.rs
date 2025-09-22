use light_compressed_account::compressed_account::PackedMerkleContext;
use light_ctoken_types::instructions::transfer2::{
    CompressedCpiContext, MultiInputTokenDataWithContext,
};
use light_profiler::profile;
use light_sdk::{
    error::LightSdkError,
    instruction::{
        account_meta::CompressedAccountMetaNoLamportsNoAddress, AccountMetasVec, PackedAccounts,
        PackedStateTreeInfo, SystemAccountMetaConfig,
    },
    token::{InputTokenDataCompressible, TokenData},
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

impl
    From<(
        InputTokenDataCompressible,
        CompressedAccountMetaNoLamportsNoAddress,
        u8,
    )> for DecompressFullIndices
{
    #[inline(never)]
    fn from(
        (token_data, meta, destination_index): (
            InputTokenDataCompressible,
            CompressedAccountMetaNoLamportsNoAddress,
            u8,
        ),
    ) -> Self {
        let source = MultiInputTokenDataWithContext {
            owner: token_data.owner,
            amount: token_data.amount,
            has_delegate: token_data.has_delegate,
            delegate: token_data.delegate,
            mint: token_data.mint,
            version: token_data.version,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: meta.tree_info.merkle_tree_pubkey_index,
                queue_pubkey_index: meta.tree_info.queue_pubkey_index,
                leaf_index: meta.tree_info.leaf_index,
                prove_by_index: meta.tree_info.prove_by_index,
            },
            root_index: meta.tree_info.root_index,
        };

        DecompressFullIndices {
            source,
            destination_index,
        }
    }
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
#[inline(never)]
#[cold]
pub fn decompress_full_ctoken_accounts_with_indices<'info>(
    fee_payer: Pubkey,
    validity_proof: ValidityProof,
    cpi_context_pubkey: Option<Pubkey>,
    indices: &[DecompressFullIndices],
    packed_accounts: &[AccountInfo<'info>],
) -> Result<Instruction, TokenSdkError> {
    spl_pod::solana_msg::msg!("decompress_full_ctoken_accounts_with_indices");
    if indices.is_empty() {
        return Err(TokenSdkError::InvalidAccountData);
    }
    // Process each set of indices
    let mut token_accounts = Vec::with_capacity(indices.len());
    spl_pod::solana_msg::msg!("allocated token_accounts vec");

    for idx in indices.iter() {
        // Create CTokenAccount2 with the source data
        // For decompress_full, we don't have an output tree since everything goes to the destination
        let mut token_account = CTokenAccount2::new(
            vec![idx.source],
            0, // No output tree for full decompress
        )?;

        // Set up decompress_full - decompress entire balance to destination ctoken account
        token_account.decompress(idx.source.amount, idx.destination_index)?;
        token_accounts.push(token_account);
    }
    spl_pod::solana_msg::msg!("pushed token_accounts ");

    // Convert packed_accounts to AccountMetas
    //
    // TODO: we may have to add conditional delegate signers for delegate
    // support via CPI.
    // Build signer flags in O(n) instead of scanning on every meta push
    let mut signer_flags = vec![false; packed_accounts.len()];

    spl_pod::solana_msg::msg!("allocated signer_flags vec");
    for idx in indices.iter() {
        let owner_idx = idx.source.owner as usize;
        if owner_idx < signer_flags.len() {
            signer_flags[owner_idx] = true;
        }
    }
    spl_pod::solana_msg::msg!("pushed signer_flags");

    let mut packed_account_metas = Vec::with_capacity(packed_accounts.len());
    spl_pod::solana_msg::msg!("allocated packed_account_metas vec");

    for (i, info) in packed_accounts.iter().enumerate() {
        packed_account_metas.push(AccountMeta {
            pubkey: *info.key,
            is_signer: info.is_signer || signer_flags[i],
            is_writable: info.is_writable,
        });
    }
    spl_pod::solana_msg::msg!("pushed packed_account_metas");

    let (meta_config, transfer_config) = if let Some(cpi_context) = cpi_context_pubkey {
        let cpi_context_config = CompressedCpiContext {
            set_context: false,
            first_set_context: false,
        };

        spl_pod::solana_msg::msg!("allocated meta_config");
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

    spl_pod::solana_msg::msg!("allocated meta_config and transfer_config");

    // Create the transfer2 instruction with all decompress operations
    // Optimization: when a CPI context is provided, the proof is already stored in the
    // CPI context (written earlier in the same transaction). To reduce instruction
    // size and runtime allocations, omit embedding the proof here.
    let effective_proof = if cpi_context_pubkey.is_some() {
        ValidityProof::new(None)
    } else {
        validity_proof
    };

    spl_pod::solana_msg::msg!("allocated effective_proof");

    let inputs = Transfer2Inputs {
        meta_config,
        token_accounts,
        transfer_config,
        validity_proof: effective_proof,
        ..Default::default()
    };
    spl_pod::solana_msg::msg!("allocated inputs");

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
#[inline(never)]
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
    #[inline(never)]
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
