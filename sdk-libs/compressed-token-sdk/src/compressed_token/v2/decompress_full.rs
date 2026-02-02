#[cfg(not(target_os = "solana"))]
use light_compressed_account::compressed_account::PackedMerkleContext;
use light_compressed_account::instruction_data::compressed_proof::ValidityProof;
use light_program_profiler::profile;
use light_sdk_types::error::LightSdkTypesError;
use light_sdk::{instruction::PackedStateTreeInfo, Unpack};
// Pack and PackedAccounts only available off-chain (client-side)
#[cfg(not(target_os = "solana"))]
use light_sdk::{
    instruction::{AccountMetasVec, PackedAccounts, SystemAccountMetaConfig},
    Pack, PackedAccountsExt,
};
use light_token_interface::instructions::{
    extensions::ExtensionInstructionData,
    transfer2::{CompressedCpiContext, MultiInputTokenDataWithContext},
};
use solana_account_info::AccountInfo;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

use super::{
    account2::CTokenAccount2,
    transfer2::{
        account_metas::Transfer2AccountsMetaConfig, create_transfer2_instruction, Transfer2Config,
        Transfer2Inputs,
    },
};
use crate::{
    compat::TokenData, error::TokenSdkError, utils::TokenDefaultAccounts, AnchorDeserialize,
    AnchorSerialize,
};

/// Struct to hold all the data needed for DecompressFull operation
/// Contains the complete compressed account data and destination index
#[derive(Debug, Clone, AnchorDeserialize, AnchorSerialize)]
pub struct DecompressFullIndices {
    pub source: MultiInputTokenDataWithContext, // Complete compressed account data with merkle context
    pub destination_index: u8,                  // Destination ctoken Solana account (must exist)
    /// Whether this is an ATA decompression. For ATAs, the source.owner is the ATA address
    /// (not the wallet), so it should NOT be marked as a signer - the wallet signs the tx instead.
    pub is_ata: bool,
    /// TLV extensions for this compressed account (e.g., CompressedOnly extension).
    /// Used to transfer extension state during decompress.
    pub tlv: Option<Vec<ExtensionInstructionData>>,
}

/// Unpacked input data for token decompression.
/// Implements `light_sdk::Pack` to produce `DecompressFullIndices`,
/// converting Pubkeys (owner, mint, delegate, destination) to u8 indices.
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct DecompressFullInput {
    pub token: TokenData,
    pub tree_info: PackedStateTreeInfo,
    pub destination: Pubkey,
    pub tlv: Option<Vec<ExtensionInstructionData>>,
    pub version: u8,
    pub is_ata: bool,
}

#[cfg(not(target_os = "solana"))]
impl Pack<AccountMeta> for DecompressFullInput {
    type Packed = DecompressFullIndices;

    fn pack(
        &self,
        remaining_accounts: &mut PackedAccounts,
    ) -> Result<Self::Packed, LightSdkTypesError> {
        let owner_is_signer = !self.is_ata;

        let source = MultiInputTokenDataWithContext {
            owner: remaining_accounts.insert_or_get_config(
                self.token.owner,
                owner_is_signer,
                false,
            ),
            amount: self.token.amount,
            has_delegate: self.token.delegate.is_some(),
            delegate: self
                .token
                .delegate
                .map(|d| remaining_accounts.insert_or_get(d))
                .unwrap_or(0),
            mint: remaining_accounts.insert_or_get(self.token.mint),
            version: self.version,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: self.tree_info.merkle_tree_pubkey_index,
                queue_pubkey_index: self.tree_info.queue_pubkey_index,
                prove_by_index: self.tree_info.prove_by_index,
                leaf_index: self.tree_info.leaf_index,
            },
            root_index: self.tree_info.root_index,
        };

        Ok(DecompressFullIndices {
            source,
            destination_index: remaining_accounts.insert_or_get(self.destination),
            tlv: self.tlv.clone(),
            is_ata: self.is_ata,
        })
    }
}

impl<'a> Unpack<AccountInfo<'a>> for DecompressFullIndices {
    type Unpacked = DecompressFullInput;

    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo<'a>],
    ) -> Result<Self::Unpacked, LightSdkTypesError> {
        let owner = *remaining_accounts
            .get(self.source.owner as usize)
            .ok_or(LightSdkTypesError::InvalidInstructionData)?
            .key;
        let mint = *remaining_accounts
            .get(self.source.mint as usize)
            .ok_or(LightSdkTypesError::InvalidInstructionData)?
            .key;
        let delegate = if self.source.has_delegate {
            Some(
                *remaining_accounts
                    .get(self.source.delegate as usize)
                    .ok_or(LightSdkTypesError::InvalidInstructionData)?
                    .key,
            )
        } else {
            None
        };
        let destination = *remaining_accounts
            .get(self.destination_index as usize)
            .ok_or(LightSdkTypesError::InvalidInstructionData)?
            .key;

        Ok(DecompressFullInput {
            token: TokenData {
                owner,
                mint,
                amount: self.source.amount,
                delegate,
                state: crate::compat::AccountState::Initialized,
                tlv: None,
            },
            tree_info: PackedStateTreeInfo {
                root_index: self.source.root_index,
                prove_by_index: self.source.merkle_context.prove_by_index,
                merkle_tree_pubkey_index: self.source.merkle_context.merkle_tree_pubkey_index,
                queue_pubkey_index: self.source.merkle_context.queue_pubkey_index,
                leaf_index: self.source.merkle_context.leaf_index,
            },
            destination,
            tlv: self.tlv.clone(),
            version: self.source.version,
            is_ata: self.is_ata,
        })
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
#[profile]
pub fn decompress_full_token_accounts_with_indices<'info>(
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
    let mut in_tlv_data: Vec<Vec<ExtensionInstructionData>> = Vec::with_capacity(indices.len());
    let mut has_any_tlv = false;

    // Convert packed_accounts to AccountMetas
    // TODO: we may have to add conditional delegate signers for delegate
    // support via CPI.
    // Build signer flags in O(n) instead of scanning on every meta push
    let mut signer_flags = vec![false; packed_accounts.len()];

    for idx in indices.iter() {
        // Create CTokenAccount2 with the source data
        // For decompress_full, we don't have an output tree since everything goes to the destination
        let mut token_account = CTokenAccount2::new(vec![idx.source])?;

        // Set up decompress_full - decompress entire balance to destination ctoken account
        token_account.decompress(idx.source.amount, idx.destination_index)?;
        token_accounts.push(token_account);

        // Collect TLV data for this input
        if let Some(tlv) = &idx.tlv {
            has_any_tlv = true;
            in_tlv_data.push(tlv.clone());
        } else {
            in_tlv_data.push(Vec::new());
        }

        let owner_idx = idx.source.owner as usize;
        if owner_idx >= signer_flags.len() {
            return Err(TokenSdkError::InvalidAccountData);
        }
        // For ATAs, the owner is the ATA address (a PDA that can't sign).
        // The wallet signs the transaction instead, so don't mark the owner as signer.
        if !idx.is_ata {
            signer_flags[owner_idx] = true;
        }
    }

    let mut packed_account_metas = Vec::with_capacity(packed_accounts.len());

    for (i, info) in packed_accounts.iter().enumerate() {
        packed_account_metas.push(AccountMeta {
            pubkey: *info.key,
            is_signer: info.is_signer || signer_flags[i],
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
        in_tlv: if has_any_tlv { Some(in_tlv_data) } else { None },
        ..Default::default()
    };

    create_transfer2_instruction(inputs)
}

/// Helper function to pack compressed token accounts into DecompressFullIndices.
/// Delegates to `DecompressFullInput::pack()`.
///
/// For non-ATA decompress: owner is marked as a signer.
#[cfg(not(target_os = "solana"))]
#[profile]
pub fn pack_for_decompress_full(
    token: &TokenData,
    tree_info: &PackedStateTreeInfo,
    destination: Pubkey,
    packed_accounts: &mut PackedAccounts,
    tlv: Option<Vec<ExtensionInstructionData>>,
    version: u8,
) -> DecompressFullIndices {
    let input = DecompressFullInput {
        token: token.clone(),
        tree_info: *tree_info,
        destination,
        tlv,
        version,
        is_ata: false,
    };
    // insert_or_get never fails, so pack is infallible for this type
    input.pack(packed_accounts).expect("infallible")
}

/// Pack accounts for decompress with ATA support.
/// Delegates to `DecompressFullInput::pack()`.
///
/// For ATA decompress (is_ata=true):
/// - Owner (ATA pubkey) is added without signer flag (ATA can't sign)
/// - Wallet owner is already added as signer by the caller
///
/// For non-ATA decompress:
/// - Owner is added as signer (normal case)
#[cfg(not(target_os = "solana"))]
#[profile]
pub fn pack_for_decompress_full_with_ata(
    token: &TokenData,
    tree_info: &PackedStateTreeInfo,
    destination: Pubkey,
    packed_accounts: &mut PackedAccounts,
    tlv: Option<Vec<ExtensionInstructionData>>,
    version: u8,
    is_ata: bool,
) -> DecompressFullIndices {
    let input = DecompressFullInput {
        token: token.clone(),
        tree_info: *tree_info,
        destination,
        tlv,
        version,
        is_ata,
    };
    input.pack(packed_accounts).expect("infallible")
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
            compressed_token_program: TokenDefaultAccounts::default().compressed_token_program,
            cpi_authority_pda: TokenDefaultAccounts::default().cpi_authority_pda,
            cpi_context,
            self_program: None,
        }
    }
    pub fn new_with_cpi_context(cpi_context: Option<Pubkey>, self_program: Option<Pubkey>) -> Self {
        Self {
            compressed_token_program: TokenDefaultAccounts::default().compressed_token_program,
            cpi_authority_pda: TokenDefaultAccounts::default().cpi_authority_pda,
            cpi_context,
            self_program,
        }
    }
}

#[cfg(not(target_os = "solana"))]
impl AccountMetasVec<AccountMeta> for DecompressFullAccounts {
    /// Adds:
    /// 1. system accounts if not set
    /// 2. compressed token program and ctoken cpi authority pda to pre accounts
    fn get_account_metas_vec(
        &self,
        accounts: &mut PackedAccounts,
    ) -> Result<(), LightSdkTypesError> {
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

            accounts
                .add_system_accounts_v2(config)
                .map_err(LightSdkTypesError::from)?;
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
