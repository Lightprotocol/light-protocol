pub mod get_compressible_account;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_client::indexer::{CompressedAccount, TreeInfo, ValidityProofWithContext};
pub use light_sdk::compressible::config::CompressibleConfig;
use light_sdk::{
    compressible::{compression_info::CompressedAccountData, Pack},
    constants::C_TOKEN_PROGRAM_ID,
    instruction::{
        account_meta::CompressedAccountMetaNoLamportsNoAddress, PackedAccounts,
        SystemAccountMetaConfig, ValidityProof,
    },
};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Helper function to get the output queue from tree info.
/// Prefers next_tree_info.queue if available, otherwise uses current queue.
#[inline]
fn get_output_queue(tree_info: &TreeInfo) -> Pubkey {
    tree_info
        .next_tree_info
        .as_ref()
        .map(|next| next.queue)
        .unwrap_or(tree_info.queue)
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeCompressionConfigData {
    pub rent_sponsor: Pubkey,
    pub address_space: Vec<Pubkey>,
    pub config_bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateCompressionConfigData {
    pub new_rent_sponsor: Option<Pubkey>,
    pub new_address_space: Option<Vec<Pubkey>>,
    pub new_update_authority: Option<Pubkey>,
}

/// T is the packed type from calling .pack() on the original type
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DecompressMultipleAccountsIdempotentData<T> {
    pub proof: ValidityProof,
    pub compressed_accounts: Vec<CompressedAccountData<T>>,
    pub system_accounts_offset: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CompressAccountsIdempotentData {
    pub proof: ValidityProof,
    pub compressed_accounts: Vec<CompressedAccountMetaNoLamportsNoAddress>,
    pub system_accounts_offset: u8,
}

/// Instruction builders for compressible accounts
pub mod compressible_instruction {
    use super::*;

    /// SHA256("global:initialize_compression_config")[..8]
    pub const INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR: [u8; 8] =
        [133, 228, 12, 169, 56, 76, 222, 61];
    /// SHA256("global:update_compression_config")[..8]
    pub const UPDATE_COMPRESSION_CONFIG_DISCRIMINATOR: [u8; 8] =
        [135, 215, 243, 81, 163, 146, 33, 70];
    /// SHA256("global:decompress_accounts_idempotent")[..8]
    pub const DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR: [u8; 8] =
        [114, 67, 61, 123, 234, 31, 1, 112];
    /// SHA256("global:compress_accounts_idempotent")[..8]
    pub const COMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR: [u8; 8] =
        [70, 236, 171, 120, 164, 93, 113, 181];

    /// Creates an initialize_compression_config instruction
    #[allow(clippy::too_many_arguments)]
    pub fn initialize_compression_config(
        program_id: &Pubkey,
        discriminator: &[u8],
        payer: &Pubkey,
        authority: &Pubkey,
        rent_sponsor: Pubkey,
        address_space: Vec<Pubkey>,
        config_bump: Option<u8>,
    ) -> Instruction {
        let config_bump = config_bump.unwrap_or(0);
        let (config_pda, _) = CompressibleConfig::derive_pda(program_id, config_bump);

        // Get program data account for BPF Loader Upgradeable
        let bpf_loader_upgradeable_id =
            solana_pubkey::pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");
        let (program_data_pda, _) =
            Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable_id);

        let system_program_id = solana_pubkey::pubkey!("11111111111111111111111111111111");
        let accounts = vec![
            AccountMeta::new(*payer, true),                      // payer
            AccountMeta::new(config_pda, false),                 // config
            AccountMeta::new_readonly(program_data_pda, false),  // program_data
            AccountMeta::new_readonly(*authority, true),         // authority
            AccountMeta::new_readonly(system_program_id, false), // system_program
        ];

        let instruction_data = InitializeCompressionConfigData {
            rent_sponsor,
            address_space,
            config_bump,
        };

        // Prepend discriminator to serialized data, following Solana SDK pattern
        let serialized_data = instruction_data
            .try_to_vec()
            .expect("Failed to serialize instruction data");

        let mut data = Vec::with_capacity(discriminator.len() + serialized_data.len());
        data.extend_from_slice(discriminator);
        data.extend_from_slice(&serialized_data);

        Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }

    /// Updates compression config
    pub fn update_compression_config(
        program_id: &Pubkey,
        discriminator: &[u8],
        authority: &Pubkey,
        new_rent_sponsor: Option<Pubkey>,
        new_address_space: Option<Vec<Pubkey>>,
        new_update_authority: Option<Pubkey>,
    ) -> Instruction {
        let (config_pda, _) = CompressibleConfig::derive_pda(program_id, 0);

        let accounts = vec![
            AccountMeta::new(config_pda, false),         // config
            AccountMeta::new_readonly(*authority, true), // authority
        ];

        let instruction_data = UpdateCompressionConfigData {
            new_rent_sponsor,
            new_address_space,
            new_update_authority,
        };

        // Prepend discriminator to serialized data, following Solana SDK pattern
        let serialized_data = instruction_data
            .try_to_vec()
            .expect("Failed to serialize instruction data");
        let mut data = Vec::with_capacity(discriminator.len() + serialized_data.len());
        data.extend_from_slice(discriminator);
        data.extend_from_slice(&serialized_data);

        Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }

    /// Builds decompress_accounts_idempotent instruction
    #[allow(clippy::too_many_arguments)]
    pub fn decompress_accounts_idempotent<T>(
        program_id: &Pubkey,
        discriminator: &[u8],
        decompressed_account_addresses: &[Pubkey],
        compressed_accounts: &[(CompressedAccount, T)],
        program_account_metas: &[AccountMeta],
        validity_proof_with_context: ValidityProofWithContext,
    ) -> Result<Instruction, Box<dyn std::error::Error>>
    where
        T: Pack + Clone + std::fmt::Debug,
    {
        if compressed_accounts.is_empty() {
            return Err("compressed_accounts cannot be empty".into());
        }

        let mut remaining_accounts = PackedAccounts::default();

        let mut has_tokens = false;
        let mut has_pdas = false;
        for (compressed_account, _) in compressed_accounts.iter() {
            if compressed_account.owner == C_TOKEN_PROGRAM_ID.into() {
                has_tokens = true;
            } else {
                has_pdas = true;
            }
            if has_tokens && has_pdas {
                break;
            }
        }
        if !has_tokens && !has_pdas {
            return Err("No tokens or PDAs found in compressed accounts".into());
        };
        if decompressed_account_addresses.len() != compressed_accounts.len() {
            return Err("PDA accounts and compressed accounts must have the same length".into());
        }

        // pack cpi_context_account if required.
        if has_pdas && has_tokens {
            let cpi_context_of_first_input =
                compressed_accounts[0].0.tree_info.cpi_context.unwrap();
            let system_config = SystemAccountMetaConfig::new_with_cpi_context(
                *program_id,
                cpi_context_of_first_input,
            );
            remaining_accounts.add_system_accounts_v2(system_config)?;
        } else {
            let system_config = SystemAccountMetaConfig::new(*program_id);
            remaining_accounts.add_system_accounts_v2(system_config)?;
        }

        // pack output queue
        let output_queue = get_output_queue(&compressed_accounts[0].0.tree_info);
        let output_state_tree_index = remaining_accounts.insert_or_get(output_queue);

        // pack tree infos
        let packed_tree_infos =
            validity_proof_with_context.pack_tree_infos(&mut remaining_accounts);

        let mut accounts = program_account_metas.to_vec();

        // pack account data
        let packed_tree_infos_slice = &packed_tree_infos
            .state_trees
            .as_ref()
            .unwrap()
            .packed_tree_infos;

        let mut typed_compressed_accounts = Vec::with_capacity(compressed_accounts.len());

        for (compressed_account, data) in compressed_accounts {
            let queue_index = remaining_accounts.insert_or_get(compressed_account.tree_info.queue);

            let tree_info = packed_tree_infos_slice
                .iter()
                .find(|pti| {
                    pti.queue_pubkey_index == queue_index
                        && pti.leaf_index == compressed_account.leaf_index
                })
                .copied()
                .ok_or(
                    "Matching PackedStateTreeInfo (queue_pubkey_index + leaf_index) not found",
                )?;

            let packed_data = data.pack(&mut remaining_accounts);
            typed_compressed_accounts.push(CompressedAccountData {
                meta: CompressedAccountMetaNoLamportsNoAddress {
                    tree_info,
                    output_state_tree_index,
                },
                data: packed_data,
            });
        }

        let (system_accounts, system_accounts_offset, _) = remaining_accounts.to_account_metas();
        accounts.extend(system_accounts);

        for account in decompressed_account_addresses {
            accounts.push(AccountMeta::new(*account, false));
        }

        let instruction_data = DecompressMultipleAccountsIdempotentData {
            proof: validity_proof_with_context.proof,
            compressed_accounts: typed_compressed_accounts,
            system_accounts_offset: system_accounts_offset as u8,
        };

        let serialized_data = instruction_data.try_to_vec()?;
        let mut data = Vec::with_capacity(discriminator.len() + serialized_data.len());
        data.extend_from_slice(discriminator);
        data.extend_from_slice(&serialized_data);

        Ok(Instruction {
            program_id: *program_id,
            accounts,
            data,
        })
    }

    /// Builds compress_accounts_idempotent instruction for PDAs only
    #[allow(clippy::too_many_arguments)]
    pub fn compress_accounts_idempotent(
        program_id: &Pubkey,
        discriminator: &[u8],
        account_pubkeys: &[Pubkey],
        accounts_to_compress: &[Account],
        program_account_metas: &[AccountMeta],
        validity_proof_with_context: ValidityProofWithContext,
    ) -> Result<Instruction, Box<dyn std::error::Error>> {
        if account_pubkeys.len() != accounts_to_compress.len() {
            return Err("Accounts pubkeys length must match accounts length".into());
        }

        if validity_proof_with_context.accounts.is_empty() {
            return Err("validity_proof_with_context.accounts cannot be empty".into());
        }

        let mut remaining_accounts = PackedAccounts::default();

        let system_config = SystemAccountMetaConfig::new(*program_id);
        remaining_accounts.add_system_accounts_v2(system_config)?;

        // pack output queue - use first tree info from validity proof
        let output_queue = get_output_queue(&validity_proof_with_context.accounts[0].tree_info);
        let output_state_tree_index = remaining_accounts.insert_or_get(output_queue);

        let packed_tree_infos =
            validity_proof_with_context.pack_tree_infos(&mut remaining_accounts);

        let mut compressed_account_metas_no_lamports_no_address = Vec::new();

        for packed_tree_info in packed_tree_infos
            .state_trees
            .as_ref()
            .unwrap()
            .packed_tree_infos
            .iter()
        {
            compressed_account_metas_no_lamports_no_address.push(
                CompressedAccountMetaNoLamportsNoAddress {
                    tree_info: *packed_tree_info,
                    output_state_tree_index,
                },
            );
        }

        let mut accounts = program_account_metas.to_vec();

        let (system_accounts, system_accounts_offset, _) = remaining_accounts.to_account_metas();
        accounts.extend(system_accounts);

        for account in account_pubkeys {
            accounts.push(AccountMeta::new(*account, false));
        }

        let instruction_data = CompressAccountsIdempotentData {
            proof: validity_proof_with_context.proof,
            compressed_accounts: compressed_account_metas_no_lamports_no_address,
            system_accounts_offset: system_accounts_offset as u8,
        };

        let serialized_data = instruction_data.try_to_vec()?;
        let mut data = Vec::with_capacity(discriminator.len() + serialized_data.len());
        data.extend_from_slice(discriminator);
        data.extend_from_slice(&serialized_data);

        Ok(Instruction {
            program_id: *program_id,
            accounts,
            data,
        })
    }
}
