pub mod account_interface;
pub mod account_interface_ext;
pub mod compressible_program;
pub mod create_accounts_proof;
pub mod decompress_mint;
pub mod get_compressible_account;
pub mod initialize_config;
pub mod load_accounts;
pub mod pack;
pub mod tx_size;

pub use account_interface::{AccountInterface, AccountInterfaceError, TokenAccountInterface};
pub use account_interface_ext::AccountInterfaceExt;
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use compressible_program::{
    all_hot, any_cold, AccountSpec, AccountToFetch, ColdContext, CompressibleProgram, PdaSpec,
};
pub use create_accounts_proof::{
    get_create_accounts_proof, CreateAccountsProofError, CreateAccountsProofInput,
    CreateAccountsProofResult,
};
pub use decompress_mint::{
    DecompressMintError, MintInterface, MintState, DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
};
pub use initialize_config::InitializeRentFreeConfig;
use light_client::indexer::{CompressedAccount, TreeInfo, ValidityProofWithContext};
pub use light_compressible::CreateAccountsProof;
pub use light_sdk::compressible::config::CompressibleConfig;
use light_sdk::{
    compressible::{compression_info::CompressedAccountData, Pack},
    instruction::{
        account_meta::CompressedAccountMetaNoLamportsNoAddress, PackedAccounts,
        SystemAccountMetaConfig, ValidityProof,
    },
};
pub use light_token_sdk::compat::TokenData;
use light_token_sdk::token::{
    COMPRESSIBLE_CONFIG_V1, LIGHT_TOKEN_CPI_AUTHORITY, LIGHT_TOKEN_PROGRAM_ID, RENT_SPONSOR,
};
pub use load_accounts::{create_load_instructions, LoadAccountsError};
pub use pack::{pack_proof, PackError, PackedProofResult};
pub use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;
pub use tx_size::{split_by_tx_size, InstructionTooLargeError, PACKET_DATA_SIZE};

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

    /// Helpers for decompress_accounts_idempotent instruction
    pub mod decompress {
        use super::*;

        /// Returns program account metas for decompress_accounts_idempotent with CToken support.
        /// Includes ctoken_rent_sponsor, light_token_program, light_token_cpi_authority, ctoken_config.
        pub fn accounts(
            fee_payer: Pubkey,
            config: Pubkey,
            rent_sponsor: Pubkey,
        ) -> Vec<AccountMeta> {
            vec![
                AccountMeta::new(fee_payer, true),
                AccountMeta::new_readonly(config, false),
                AccountMeta::new(rent_sponsor, false),
                AccountMeta::new(RENT_SPONSOR, false),
                AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(LIGHT_TOKEN_CPI_AUTHORITY, false),
                AccountMeta::new_readonly(COMPRESSIBLE_CONFIG_V1, false),
            ]
        }

        /// Returns program account metas for PDA-only decompression (no CToken accounts).
        /// Note: Still passes all 7 accounts because the struct has Optional fields that
        /// Anchor still deserializes. Uses rent_sponsor as placeholder for ctoken_rent_sponsor.
        pub fn accounts_pda_only(
            fee_payer: Pubkey,
            config: Pubkey,
            rent_sponsor: Pubkey,
        ) -> Vec<AccountMeta> {
            vec![
                AccountMeta::new(fee_payer, true),
                AccountMeta::new_readonly(config, false),
                AccountMeta::new(rent_sponsor, false),
                // Optional token accounts - use placeholders that satisfy constraints
                AccountMeta::new(rent_sponsor, false), // ctoken_rent_sponsor (mut) - reuse rent_sponsor
                AccountMeta::new_readonly(LIGHT_TOKEN_PROGRAM_ID, false),
                AccountMeta::new_readonly(LIGHT_TOKEN_CPI_AUTHORITY, false),
                AccountMeta::new_readonly(COMPRESSIBLE_CONFIG_V1, false),
            ]
        }
    }

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

    /// Builds decompress_accounts_idempotent instruction (raw version with explicit discriminator)
    #[allow(clippy::too_many_arguments)]
    pub fn build_decompress_idempotent_raw<T>(
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
            if compressed_account.owner == LIGHT_TOKEN_PROGRAM_ID {
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
        // CRITICAL: When both PDAs and tokens exist, tokens execute LAST (consuming the CPI context).
        // CPI context validation checks: cpi_context.associated_tree == first_input_of_executor.tree
        // So we must use the FIRST TOKEN's cpi_context, not the first PDA's.
        if has_pdas && has_tokens {
            // Find the first token account's CPI context
            let first_token_cpi_context = compressed_accounts
                .iter()
                .find(|(acc, _)| acc.owner == LIGHT_TOKEN_PROGRAM_ID)
                .map(|(acc, _)| acc.tree_info.cpi_context.unwrap())
                .expect("has_tokens is true so there must be a token");
            let system_config =
                SystemAccountMetaConfig::new_with_cpi_context(*program_id, first_token_cpi_context);
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

        // The compressed_accounts are expected to be in the SAME ORDER as the
        // validity_proof_with_context.accounts. This is because both are derived
        // from the same hash order passed to get_validity_proof().
        // We use index-based matching instead of queue+leaf_index to handle
        // accounts on different trees with potentially colliding indices.
        for (i, (compressed_account, data)) in compressed_accounts.iter().enumerate() {
            // Insert the queue for this account (needed for the packed context)
            let _queue_index = remaining_accounts.insert_or_get(compressed_account.tree_info.queue);

            // Use index-based matching - the i-th compressed account uses the i-th tree info
            let tree_info = packed_tree_infos_slice.get(i).copied().ok_or(
                "Tree info index out of bounds - compressed_accounts length must match validity proof accounts length",
            )?;

            let packed_data = data.pack(&mut remaining_accounts)?;
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
