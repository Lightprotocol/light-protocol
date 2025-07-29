#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_client::indexer::{CompressedAccount, TreeInfo, ValidityProofWithContext};
pub use light_sdk::compressible::config::CompressibleConfig;
use light_sdk::instruction::{
    account_meta::CompressedAccountMeta, PackedAccounts, SystemAccountMetaConfig, ValidityProof,
};
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Generic compressed account data structure for decompress operations
/// This is generic over the account variant type, allowing programs to use their specific enums
///
/// # Type Parameters
/// * `T` - The program-specific compressed account variant enum (e.g., CompressedAccountVariant)
///
/// # Fields
/// * `meta` - The compressed account metadata containing tree info, address, and output index
/// * `data` - The program-specific account variant enum
/// * `seeds` - The PDA seeds (without bump) used to derive the PDA address
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct CompressedAccountData<T> {
    pub meta: CompressedAccountMeta,
    /// Program-specific account variant enum
    pub data: T,
    /// PDA seeds (without bump) used to derive the PDA address
    pub seeds: Vec<Vec<u8>>,
}

/// Instruction data structure for decompress_accounts_idempotent
/// This matches the exact format expected by Anchor programs
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct DecompressMultipleAccountsIdempotentData<T> {
    pub proof: ValidityProof,
    pub compressed_accounts: Vec<CompressedAccountData<T>>,
    pub bumps: Vec<u8>,
    pub system_accounts_offset: u8,
}

/// Instruction builders for compressible accounts, following Solana SDK patterns
/// These are generic builders that work with any program implementing the compressible pattern
pub struct CompressibleInstruction;

impl CompressibleInstruction {
    pub const INITIALIZE_COMPRESSION_CONFIG_DISCRIMINATOR: [u8; 8] =
        [133, 228, 12, 169, 56, 76, 222, 61];
    pub const UPDATE_COMPRESSION_CONFIG_DISCRIMINATOR: [u8; 8] =
        [135, 215, 243, 81, 163, 146, 33, 70];
    /// Hardcoded discriminator for the standardized decompress_accounts_idempotent instruction
    /// This is calculated as SHA256("global:decompress_accounts_idempotent")[..8] (Anchor format)
    pub const DECOMPRESS_ACCOUNTS_IDEMPOTENT_DISCRIMINATOR: [u8; 8] =
        [114, 67, 61, 123, 234, 31, 1, 112];

    /// Creates an initialize_compression_config instruction
    ///
    /// Following Solana SDK patterns like system_instruction::transfer()
    /// Returns Instruction directly - errors surface at execution time
    ///
    /// # Arguments
    /// * `program_id` - The program ID
    /// * `discriminator` - The instruction discriminator bytes (flexible length)
    /// * `payer` - The payer account
    /// * `authority` - The authority account
    /// * `compression_delay` - The compression delay
    /// * `rent_recipient` - The rent recipient
    /// * `address_space` - The address space
    /// * `config_bump` - The config bump
    #[allow(clippy::too_many_arguments)]
    pub fn initialize_compression_config(
        program_id: &Pubkey,
        discriminator: &[u8],
        payer: &Pubkey,
        authority: &Pubkey,
        compression_delay: u32,
        rent_recipient: Pubkey,
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
            compression_delay,
            rent_recipient,
            address_space,
            config_bump,
        };

        // Prepend discriminator to serialized data, following Solana SDK pattern
        let serialized_data = instruction_data
            .try_to_vec()
            .expect("Failed to serialize instruction data");

        let mut data = Vec::new();
        data.extend_from_slice(discriminator);
        data.extend_from_slice(&serialized_data);

        Instruction {
            program_id: *program_id,
            accounts,
            data,
        }
    }

    /// Creates an update config instruction
    ///
    /// Following Solana SDK patterns - returns Instruction directly
    ///
    /// # Arguments
    /// * `program_id` - The program ID
    /// * `discriminator` - The instruction discriminator bytes (flexible length)
    /// * `authority` - The authority account
    /// * `new_compression_delay` - Optional new compression delay
    /// * `new_rent_recipient` - Optional new rent recipient
    /// * `new_address_space` - Optional new address space
    /// * `new_update_authority` - Optional new update authority
    pub fn update_compression_config(
        program_id: &Pubkey,
        discriminator: &[u8],
        authority: &Pubkey,
        new_compression_delay: Option<u32>,
        new_rent_recipient: Option<Pubkey>,
        new_address_space: Option<Vec<Pubkey>>,
        new_update_authority: Option<Pubkey>,
    ) -> Instruction {
        let (config_pda, _) = CompressibleConfig::derive_pda(program_id, 0);

        let accounts = vec![
            AccountMeta::new(config_pda, false),         // config
            AccountMeta::new_readonly(*authority, true), // authority
        ];

        let instruction_data = UpdateCompressionConfigData {
            new_compression_delay,
            new_rent_recipient,
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

    /// Creates a generic compress account instruction for any compressible account
    ///
    /// This is a generic helper that can be used by any program client to build
    /// a compress account instruction. The caller must provide the instruction
    /// discriminator specific to their program.
    ///
    /// # Arguments
    /// * `program_id` - The program that owns the compressible account
    /// * `discriminator` - The instruction discriminator bytes (flexible length)
    /// * `payer` - The account paying for the transaction
    /// * `pda_to_compress` - The PDA account to compress
    /// * `rent_recipient` - The account to receive the reclaimed rent
    /// * `compressed_account` - The compressed account to be nullified
    /// * `validity_proof_with_context` - The validity proof with context from the indexer
    /// * `output_state_tree_info` - The output state tree info
    ///
    /// # Returns
    /// * `Result<Instruction, Box<dyn std::error::Error>>` - The complete instruction ready to be sent
    #[allow(clippy::too_many_arguments)]
    pub fn compress_account(
        program_id: &Pubkey,
        discriminator: &[u8],
        payer: &Pubkey,
        pda_to_compress: &Pubkey,
        rent_recipient: &Pubkey,
        compressed_account: &CompressedAccount,
        validity_proof_with_context: ValidityProofWithContext,
        output_state_tree_info: TreeInfo,
    ) -> Result<Instruction, Box<dyn std::error::Error>> {
        let config_pda = CompressibleConfig::derive_pda(program_id, 0).0;

        // Create system accounts internally (same pattern as decompress_accounts_idempotent)
        let mut remaining_accounts = PackedAccounts::default();
        let system_config = SystemAccountMetaConfig::new(*program_id);
        remaining_accounts.add_system_accounts(system_config);

        // Pack tree infos into remaining accounts
        let packed_tree_infos =
            validity_proof_with_context.pack_tree_infos(&mut remaining_accounts);

        // Get output state tree index
        let output_state_tree_index =
            remaining_accounts.insert_or_get(output_state_tree_info.queue);

        // Find the tree info index for this compressed account's queue
        let queue_index = remaining_accounts.insert_or_get(compressed_account.tree_info.queue);

        // Create compressed account meta
        let compressed_account_meta = CompressedAccountMeta {
            tree_info: packed_tree_infos
                .state_trees
                .as_ref()
                .unwrap()
                .packed_tree_infos
                .iter()
                .find(|pti| {
                    pti.queue_pubkey_index == queue_index
                        && pti.leaf_index == compressed_account.leaf_index
                })
                .copied()
                .ok_or(
                    "Matching PackedStateTreeInfo (queue_pubkey_index + leaf_index) not found",
                )?,
            address: compressed_account.address.unwrap_or([0u8; 32]),
            output_state_tree_index,
        };

        // Get system accounts for the instruction
        let (system_accounts, _, _) = remaining_accounts.to_account_metas();

        // Create the instruction account metas
        let accounts = vec![
            AccountMeta::new(*payer, true),               // user (signer)
            AccountMeta::new(*pda_to_compress, false),    // pda_to_compress (writable)
            AccountMeta::new_readonly(config_pda, false), // config
            AccountMeta::new(*rent_recipient, false),     // rent_recipient (writable)
        ];

        // Create instruction data
        let instruction_data = GenericCompressAccountInstruction {
            proof: validity_proof_with_context.proof,
            compressed_account_meta,
        };

        // Manually serialize instruction data with discriminator
        let serialized_data = instruction_data
            .try_to_vec()
            .expect("Failed to serialize instruction data");
        let mut data = Vec::new();
        data.extend_from_slice(discriminator);
        data.extend_from_slice(&serialized_data);

        // Build the instruction
        Ok(Instruction {
            program_id: *program_id,
            accounts: [accounts, system_accounts].concat(),
            data,
        })
    }

    /// Build a `decompress_accounts_idempotent` instruction for any program's compressed account variant.
    ///
    /// # Arguments
    /// * `program_id` - Target program
    /// * `discriminator` - The instruction discriminator bytes (flexible length)
    /// * `fee_payer` - Fee payer signer
    /// * `rent_payer` - Rent payer signer
    /// * `solana_accounts` - PDAs to decompress into
    /// * `compressed_accounts` - (meta, variant, seeds) tuples where seeds are PDA seeds without bump
    /// * `bumps` - PDA bump seeds
    /// * `validity_proof_with_context` - Validity proof with context
    /// * `output_state_tree_info` - Output state tree info
    ///
    /// Returns `Ok(Instruction)` or error.
    #[allow(clippy::too_many_arguments)]
    pub fn decompress_accounts_idempotent<T>(
        program_id: &Pubkey,
        discriminator: &[u8],
        fee_payer: &Pubkey,
        rent_payer: &Pubkey,
        solana_accounts: &[Pubkey],
        compressed_accounts: &[(CompressedAccount, T, Vec<Vec<u8>>)],
        bumps: &[u8],
        validity_proof_with_context: ValidityProofWithContext,
        output_state_tree_info: TreeInfo,
    ) -> Result<Instruction, Box<dyn std::error::Error>>
    where
        T: AnchorSerialize + Clone + std::fmt::Debug,
    {
        // Setup remaining accounts to get tree infos
        let mut remaining_accounts = PackedAccounts::default();
        let system_config = SystemAccountMetaConfig::new(*program_id);
        remaining_accounts.add_system_accounts(system_config);

        for pda in solana_accounts {
            remaining_accounts.add_pre_accounts_meta(AccountMeta::new(*pda, false));
        }

        let packed_tree_infos =
            validity_proof_with_context.pack_tree_infos(&mut remaining_accounts);

        // get output state tree index
        let output_state_tree_index =
            remaining_accounts.insert_or_get(output_state_tree_info.queue);

        // Validation
        if solana_accounts.len() != compressed_accounts.len() {
            return Err("PDA accounts and compressed accounts must have the same length".into());
        }
        if solana_accounts.len() != bumps.len() {
            return Err("PDA accounts and bumps must have the same length".into());
        }

        let config_pda = CompressibleConfig::derive_pda(program_id, 0).0;

        // Build instruction accounts
        let mut accounts = vec![
            AccountMeta::new(*fee_payer, true),           // fee_payer
            AccountMeta::new(*rent_payer, true),          // rent_payer
            AccountMeta::new_readonly(config_pda, false), // config
        ];

        // Add Light Protocol system accounts (already packed by caller)
        let (system_accounts, _, _) = remaining_accounts.to_account_metas();
        accounts.extend(system_accounts);

        // Convert to typed compressed account data
        let typed_compressed_accounts: Vec<CompressedAccountData<T>> = compressed_accounts
            .iter()
            .map(|(compressed_account, data, seeds)| {
                // Find the tree info index for this compressed account's queue
                let queue_index =
                    remaining_accounts.insert_or_get(compressed_account.tree_info.queue);
                    let compressed_meta = CompressedAccountMeta {
                    // TODO: Find cleaner way to do this.   
                    tree_info: packed_tree_infos
                        .state_trees
                        .as_ref()
                        .unwrap()
                        .packed_tree_infos
                        .iter()
                        .find(|pti| {
                            pti.queue_pubkey_index == queue_index
                                && pti.leaf_index == compressed_account.leaf_index
                        })
                        .copied()
                        .ok_or("Matching PackedStateTreeInfo (queue_pubkey_index + leaf_index) not found")?,
                    address: compressed_account.address.unwrap_or([0u8; 32]),
                    output_state_tree_index,
                };
                Ok(CompressedAccountData {
                    meta: compressed_meta,
                    data: data.clone(),
                    seeds: seeds.clone(),
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;

        // Build instruction data
        let instruction_data = DecompressMultipleAccountsIdempotentData {
            proof: validity_proof_with_context.proof,
            compressed_accounts: typed_compressed_accounts,
            bumps: bumps.to_vec(),
            system_accounts_offset: solana_accounts.len() as u8,
        };

        // Serialize instruction data with discriminator
        let serialized_data = instruction_data.try_to_vec()?;
        let mut data = Vec::new();
        data.extend_from_slice(discriminator);
        data.extend_from_slice(&serialized_data);

        Ok(Instruction {
            program_id: *program_id,
            accounts,
            data,
        })
    }
}

/// Generic instruction data for initialize config
/// Note: Real programs should use their specific instruction format
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeCompressionConfigData {
    pub compression_delay: u32,
    pub rent_recipient: Pubkey,
    pub address_space: Vec<Pubkey>,
    pub config_bump: u8,
}

/// Generic instruction data for update config
/// Note: Real programs should use their specific instruction format  
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct UpdateCompressionConfigData {
    pub new_compression_delay: Option<u32>,
    pub new_rent_recipient: Option<Pubkey>,
    pub new_address_space: Option<Vec<Pubkey>>,
    pub new_update_authority: Option<Pubkey>,
}

/// Generic instruction data for compress account
/// This matches the expected format for compress account instructions
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct GenericCompressAccountInstruction {
    pub proof: ValidityProof,
    pub compressed_account_meta: CompressedAccountMeta,
}

/// Generic instruction data for decompress multiple PDAs
// Re-export for easy access following Solana SDK patterns
pub use CompressibleInstruction as compressible_instruction;
