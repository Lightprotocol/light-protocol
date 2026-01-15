pub mod get_compressible_account;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
use light_client::indexer::{
    CompressedAccount, CompressedTokenAccount, TreeInfo, ValidityProofWithContext,
};
use light_ctoken_sdk::compat::AccountState;
pub use light_ctoken_sdk::compressible::{LightAta, LightMint};
pub use light_sdk::compressible::config::CompressibleConfig;
use light_sdk::{
    compressible::{compression_info::CompressedAccountData, Pack, StandardCompressedVariant},
    constants::C_TOKEN_PROGRAM_ID,
    instruction::{
        account_meta::CompressedAccountMetaNoLamportsNoAddress, PackedAccounts,
        SystemAccountMetaConfig, ValidityProof,
    },
};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

/// Input type for decompress_accounts_idempotent instruction builder.
/// Allows mixing PDAs with standard ATAs and Mints.
pub enum DecompressInput<T> {
    /// Program-specific PDA or CToken account
    ProgramData(CompressedAccount, T),
    /// Standard ATA from compressed token account
    Ata(CompressedTokenAccount),
    /// Standard CMint from compressed mint
    Mint {
        compressed_account: CompressedAccount,
        mint_seed_pubkey: Pubkey,
        rent_payment: u8,
        write_top_up: u32,
    },
}

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

    /// Builds decompress_accounts_idempotent instruction with unified inputs.
    ///
    /// Supports mixing:
    /// - Program PDAs via `DecompressInput::ProgramData`
    /// - Standard ATAs via `DecompressInput::Ata`
    /// - Standard CMints via `DecompressInput::Mint`
    ///
    /// # Constraints (validated at client + runtime):
    /// - At most 1 mint per instruction
    /// - Mint + (ATA/CToken) combination is forbidden
    /// - Mint + PDAs is allowed
    /// - Any combination of ATAs, CTokens, and PDAs works
    ///
    /// # Arguments
    /// * `decompressed_account_addresses` - On-chain addresses for decompression targets
    ///   (PDAs, ATAs, CMint PDAs in same order as inputs)
    #[allow(clippy::too_many_arguments)]
    pub fn decompress_accounts_unified<T>(
        program_id: &Pubkey,
        discriminator: &[u8],
        decompressed_account_addresses: &[Pubkey],
        inputs: Vec<DecompressInput<T>>,
        program_account_metas: &[AccountMeta],
        validity_proof_with_context: ValidityProofWithContext,
    ) -> Result<Instruction, Box<dyn std::error::Error>>
    where
        T: Pack + StandardCompressedVariant + Clone + std::fmt::Debug,
    {
        if inputs.is_empty() {
            return Err("inputs cannot be empty".into());
        }
        if decompressed_account_addresses.len() != inputs.len() {
            return Err("decompressed_account_addresses length must match inputs length".into());
        }

        // Count types for validation and CPI context decision
        let mut has_tokens = false;
        let mut has_pdas = false;
        let mut mint_count = 0;

        for input in &inputs {
            match input {
                DecompressInput::ProgramData(compressed, _) => {
                    if compressed.owner == C_TOKEN_PROGRAM_ID.into() {
                        has_tokens = true;
                    } else {
                        has_pdas = true;
                    }
                }
                DecompressInput::Ata(_) => has_tokens = true,
                DecompressInput::Mint { .. } => mint_count += 1,
            }
        }
        let has_mints = mint_count > 0;

        // Client-side validation (matches runtime validation)
        if mint_count > 1 {
            return Err("At most 1 mint allowed per instruction".into());
        }
        if has_mints && has_tokens {
            return Err("Mint + (ATA/CToken) combination is forbidden".into());
        }
        if !has_tokens && !has_pdas && !has_mints {
            return Err("No tokens, PDAs, or mints found".into());
        }

        let mut remaining_accounts = PackedAccounts::default();

        // Determine CPI context needs
        let type_count = has_tokens as u8 + has_pdas as u8 + has_mints as u8;
        let needs_cpi_context = type_count >= 2;

        if needs_cpi_context {
            let cpi_context_of_first = match &inputs[0] {
                DecompressInput::ProgramData(c, _) => c.tree_info.cpi_context,
                DecompressInput::Ata(c) => c.account.tree_info.cpi_context,
                DecompressInput::Mint {
                    compressed_account, ..
                } => compressed_account.tree_info.cpi_context,
            };
            if let Some(cpi_ctx) = cpi_context_of_first {
                let system_config =
                    SystemAccountMetaConfig::new_with_cpi_context(*program_id, cpi_ctx);
                remaining_accounts.add_system_accounts_v2(system_config)?;
            } else {
                return Err("CPI context required for mixed types but not available".into());
            }
        } else {
            let system_config = SystemAccountMetaConfig::new(*program_id);
            remaining_accounts.add_system_accounts_v2(system_config)?;
        }

        // Get output queue from first input
        let first_tree_info = match &inputs[0] {
            DecompressInput::ProgramData(c, _) => &c.tree_info,
            DecompressInput::Ata(c) => &c.account.tree_info,
            DecompressInput::Mint {
                compressed_account, ..
            } => &compressed_account.tree_info,
        };
        let output_queue = get_output_queue(first_tree_info);
        let output_state_tree_index = remaining_accounts.insert_or_get(output_queue);

        // Pack tree infos
        let packed_tree_infos =
            validity_proof_with_context.pack_tree_infos(&mut remaining_accounts);
        let packed_tree_infos_slice = &packed_tree_infos
            .state_trees
            .as_ref()
            .unwrap()
            .packed_tree_infos;

        let mut accounts = program_account_metas.to_vec();
        let mut typed_compressed_accounts: Vec<CompressedAccountData<T::Packed>> =
            Vec::with_capacity(inputs.len());

        for (i, input) in inputs.into_iter().enumerate() {
            let tree_info = packed_tree_infos_slice.get(i).copied().ok_or(
                "Tree info index out of bounds - inputs length must match validity proof accounts length",
            )?;

            match input {
                DecompressInput::ProgramData(compressed_account, data) => {
                    let _queue_index =
                        remaining_accounts.insert_or_get(compressed_account.tree_info.queue);
                    let packed_data = data.pack(&mut remaining_accounts);
                    typed_compressed_accounts.push(CompressedAccountData {
                        meta: CompressedAccountMetaNoLamportsNoAddress {
                            tree_info,
                            output_state_tree_index,
                        },
                        data: packed_data,
                    });
                }
                DecompressInput::Ata(compressed_token) => {
                    let _queue_index =
                        remaining_accounts.insert_or_get(compressed_token.account.tree_info.queue);

                    // Pack wallet (owner), mint, and derived ATA
                    let wallet_pubkey =
                        Pubkey::new_from_array(compressed_token.token.owner.to_bytes());
                    let mint_pubkey =
                        Pubkey::new_from_array(compressed_token.token.mint.to_bytes());

                    // Derive ATA address
                    let (ata_address, _) =
                        light_ctoken_sdk::ctoken::get_associated_ctoken_address_and_bump(
                            &wallet_pubkey,
                            &mint_pubkey,
                        );

                    let wallet_index =
                        remaining_accounts.insert_or_get_config(wallet_pubkey, true, false);
                    let mint_index = remaining_accounts.insert_or_get_read_only(mint_pubkey);
                    let ata_index = remaining_accounts.insert_or_get(ata_address);

                    let has_delegate = compressed_token.token.delegate.is_some();
                    let delegate_index = if let Some(delegate) = compressed_token.token.delegate {
                        remaining_accounts
                            .insert_or_get_read_only(Pubkey::new_from_array(delegate.to_bytes()))
                    } else {
                        0
                    };

                    let light_ata = LightAta {
                        wallet_index,
                        mint_index,
                        ata_index,
                        amount: compressed_token.token.amount,
                        has_delegate,
                        delegate_index,
                        is_frozen: compressed_token.token.state == AccountState::Frozen,
                    };

                    // Use StandardCompressedVariant trait to pack LightAta
                    let packed_data = T::pack_light_ata(light_ata);
                    typed_compressed_accounts.push(CompressedAccountData {
                        meta: CompressedAccountMetaNoLamportsNoAddress {
                            tree_info,
                            output_state_tree_index,
                        },
                        data: packed_data,
                    });
                }
                DecompressInput::Mint {
                    compressed_account,
                    mint_seed_pubkey,
                    rent_payment,
                    write_top_up,
                } => {
                    let _queue_index =
                        remaining_accounts.insert_or_get(compressed_account.tree_info.queue);

                    // Derive CMint PDA
                    let (cmint_pda, _) =
                        light_ctoken_sdk::ctoken::find_cmint_address(&mint_seed_pubkey);

                    let mint_seed_index =
                        remaining_accounts.insert_or_get_read_only(mint_seed_pubkey);
                    let cmint_pda_index = remaining_accounts.insert_or_get(cmint_pda);

                    // Parse the compressed mint data to extract authorities
                    let mint_data: light_ctoken_interface::state::CompressedMint =
                        borsh::BorshDeserialize::deserialize(
                            &mut &compressed_account
                                .data
                                .as_ref()
                                .ok_or("Compressed mint must have data")?
                                .data[..],
                        )
                        .map_err(|e| format!("Failed to parse mint data: {}", e))?;

                    let has_mint_authority = mint_data.base.mint_authority.is_some();
                    let mint_authority_index = if let Some(auth) = mint_data.base.mint_authority {
                        remaining_accounts
                            .insert_or_get_read_only(Pubkey::new_from_array(auth.to_bytes()))
                    } else {
                        0
                    };

                    let has_freeze_authority = mint_data.base.freeze_authority.is_some();
                    let freeze_authority_index = if let Some(auth) = mint_data.base.freeze_authority
                    {
                        remaining_accounts
                            .insert_or_get_read_only(Pubkey::new_from_array(auth.to_bytes()))
                    } else {
                        0
                    };

                    let light_mint = LightMint {
                        mint_seed_index,
                        cmint_pda_index,
                        has_mint_authority,
                        mint_authority_index,
                        has_freeze_authority,
                        freeze_authority_index,
                        compressed_address: mint_data.metadata.compressed_address,
                        decimals: mint_data.base.decimals,
                        supply: mint_data.base.supply,
                        version: mint_data.metadata.version,
                        cmint_decompressed: mint_data.metadata.cmint_decompressed,
                        rent_payment,
                        write_top_up,
                        extensions: None, // TODO: support extensions
                    };

                    // Use StandardCompressedVariant trait to pack LightMint
                    let packed_data = T::pack_light_mint(light_mint);
                    typed_compressed_accounts.push(CompressedAccountData {
                        meta: CompressedAccountMetaNoLamportsNoAddress {
                            tree_info,
                            output_state_tree_index,
                        },
                        data: packed_data,
                    });
                }
            }
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
