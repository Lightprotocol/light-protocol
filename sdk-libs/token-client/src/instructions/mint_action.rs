use borsh::BorshDeserialize;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_account::instruction_data::traits::LightInstructionData;
use light_compressible::config::CompressibleConfig;
use light_token_interface::{
    instructions::{
        extensions::{token_metadata::TokenMetadataInstructionData, ExtensionInstructionData},
        mint_action::{
            CompressAndCloseCMintAction, CompressedMintWithContext, DecompressMintAction,
            MintActionCompressedInstructionData, MintToTokenAction, MintToCompressedAction,
            Recipient, RemoveMetadataKeyAction, UpdateAuthority, UpdateMetadataAuthorityAction,
            UpdateMetadataFieldAction,
        },
    },
    state::CompressedMint,
    LIGHT_TOKEN_PROGRAM_ID,
};
use light_ctoken_sdk::compressed_token::{
    create_compressed_mint::{derive_cmint_compressed_address, find_cmint_address},
    mint_action::MintActionMetaConfig,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

// Backwards compatibility types for token-client
#[derive(Debug, Clone, PartialEq)]
pub struct MintToRecipient {
    pub recipient: Pubkey,
    pub amount: u64,
}

/// High-level action types for the mint action instruction (backwards compatibility)
#[derive(Debug, Clone, PartialEq)]
pub enum MintActionType {
    MintTo {
        recipients: Vec<MintToRecipient>,
        token_account_version: u8,
    },
    UpdateMintAuthority {
        new_authority: Option<Pubkey>,
    },
    UpdateFreezeAuthority {
        new_authority: Option<Pubkey>,
    },
    MintToCToken {
        account: Pubkey,
        amount: u64,
    },
    UpdateMetadataField {
        extension_index: u8,
        field_type: u8,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    UpdateMetadataAuthority {
        extension_index: u8,
        new_authority: Pubkey,
    },
    RemoveMetadataKey {
        extension_index: u8,
        key: Vec<u8>,
        idempotent: u8,
    },
    /// Decompress the compressed mint to a CMint Solana account.
    /// CMint is always compressible - rent_payment must be >= 2.
    DecompressMint {
        cmint_bump: u8,
        /// Rent payment in epochs (prepaid). Must be >= 2.
        rent_payment: u8,
        /// Lamports allocated for future write operations (top-up per write).
        write_top_up: u32,
    },
    /// Compress and close a CMint Solana account. The compressed mint state is preserved.
    /// Permissionless - anyone can call if is_compressible() returns true (rent expired).
    CompressAndCloseCMint {
        /// If true, succeed silently when CMint doesn't exist
        idempotent: bool,
    },
}

/// Parameters for creating a new mint
#[derive(Debug, Clone)]
pub struct NewMint {
    pub decimals: u8,
    pub supply: u64,
    pub mint_authority: Pubkey,
    pub freeze_authority: Option<Pubkey>,
    pub metadata: Option<TokenMetadataInstructionData>,
    pub version: u8,
}

/// Parameters for mint action instruction
#[derive(Debug, Clone)]
pub struct MintActionParams {
    pub compressed_mint_address: [u8; 32],
    pub mint_seed: Pubkey,
    pub authority: Pubkey,
    pub payer: Pubkey,
    pub actions: Vec<MintActionType>,
    /// Required if any action is creating a mint
    pub new_mint: Option<NewMint>,
}

/// Creates a mint action instruction that can perform multiple mint operations
pub async fn create_mint_action_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    params: MintActionParams,
) -> Result<Instruction, RpcError> {
    // Check if we're creating a new mint
    let is_creating_mint = params.new_mint.is_some();

    // Check if DecompressMint action is present
    let has_decompress_mint = params
        .actions
        .iter()
        .any(|a| matches!(a, MintActionType::DecompressMint { .. }));

    // Check if CompressAndCloseCMint action is present
    let has_compress_and_close_cmint = params
        .actions
        .iter()
        .any(|a| matches!(a, MintActionType::CompressAndCloseCMint { .. }));

    // Get address tree and output queue info
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let (compressed_mint_inputs, proof, state_tree_info) = if is_creating_mint {
        let state_tree_info = rpc.get_random_state_tree_info()?;
        // For creating mint: get address proof and create placeholder compressed mint inputs
        let rpc_proof_result = rpc
            .get_validity_proof(
                vec![],
                vec![light_client::indexer::AddressWithTree {
                    address: params.compressed_mint_address,
                    tree: address_tree_pubkey,
                }],
                None,
            )
            .await?
            .value;

        // Create compressed mint data for creation with actual values
        let new_mint = params.new_mint.as_ref().ok_or_else(|| {
            RpcError::CustomError("NewMint parameters required for mint creation".to_string())
        })?;

        let mint_data =
            light_token_interface::instructions::mint_action::CompressedMintInstructionData {
                supply: new_mint.supply,
                decimals: new_mint.decimals,
                metadata: light_token_interface::state::CompressedMintMetadata {
                    version: new_mint.version,
                    mint: find_cmint_address(&params.mint_seed).0.to_bytes().into(),
                    // false for new mint - on-chain sets to true after DecompressMint
                    cmint_decompressed: false,
                    compressed_address: params.compressed_mint_address,
                },
                mint_authority: Some(new_mint.mint_authority.to_bytes().into()),
                freeze_authority: new_mint.freeze_authority.map(|auth| auth.to_bytes().into()),
                extensions: new_mint
                    .metadata
                    .as_ref()
                    .map(|meta| vec![ExtensionInstructionData::TokenMetadata(meta.clone())]),
            };

        let compressed_mint_inputs = CompressedMintWithContext {
            prove_by_index: false, // Use full proof for creation
            leaf_index: 0,         // Not applicable for creation
            root_index: rpc_proof_result.addresses[0].root_index,
            address: params.compressed_mint_address,
            mint: Some(mint_data),
        };

        (
            compressed_mint_inputs,
            rpc_proof_result.proof.0,
            state_tree_info,
        )
    } else {
        // For existing mint: get validity proof for the compressed mint
        let compressed_mint_account = rpc
            .get_compressed_account(params.compressed_mint_address, None)
            .await?
            .value
            .ok_or(RpcError::AccountDoesNotExist(format!(
                "{:?}",
                params.compressed_mint_address
            )))?;

        // Try to deserialize the compressed mint - may be None if CMint is source of truth
        let compressed_mint: Option<CompressedMint> = compressed_mint_account
            .data
            .as_ref()
            .and_then(|d| BorshDeserialize::deserialize(&mut d.data.as_slice()).ok());

        let rpc_proof_result = rpc
            .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
            .await?
            .value;

        let compressed_mint_inputs = CompressedMintWithContext {
            prove_by_index: rpc_proof_result.accounts[0].root_index.proof_by_index(),
            leaf_index: compressed_mint_account.leaf_index,
            root_index: rpc_proof_result.accounts[0]
                .root_index
                .root_index()
                .unwrap_or_default(),
            address: params.compressed_mint_address,
            mint: compressed_mint.map(|m| m.try_into().unwrap()),
        };

        (
            compressed_mint_inputs,
            rpc_proof_result.proof.into(),
            rpc_proof_result.accounts[0].tree_info,
        )
    };

    // Build instruction data using builder pattern
    let mut instruction_data = if is_creating_mint {
        MintActionCompressedInstructionData::new_mint(
            compressed_mint_inputs.root_index,
            proof.ok_or_else(|| {
                RpcError::CustomError("Proof is required for mint creation".to_string())
            })?,
            compressed_mint_inputs.mint.unwrap().clone(),
        )
    } else {
        MintActionCompressedInstructionData::new(compressed_mint_inputs.clone(), proof)
    };

    // Convert and add actions using builder pattern
    // Collect decompressed token accounts for MintToCToken actions
    let mut ctoken_accounts = Vec::new();
    let mut ctoken_account_index = 0u8;
    let mut has_mint_to_compressed = false;

    for action in params.actions {
        instruction_data = match action {
            MintActionType::MintTo {
                recipients,
                token_account_version,
            } => {
                has_mint_to_compressed = true;
                // Convert MintToRecipient (solana_sdk::Pubkey) to Recipient ([u8; 32])
                let ctoken_recipients: Vec<Recipient> = recipients
                    .into_iter()
                    .map(|r| Recipient::new(r.recipient, r.amount))
                    .collect();
                instruction_data.with_mint_to_compressed(MintToCompressedAction {
                    token_account_version,
                    recipients: ctoken_recipients,
                })
            }
            MintActionType::MintToCToken { account, amount } => {
                // Add account to the list and use its index
                ctoken_accounts.push(account);
                let current_index = ctoken_account_index;
                ctoken_account_index += 1;

                instruction_data.with_mint_to_token(MintToTokenAction {
                    account_index: current_index,
                    amount,
                })
            }
            MintActionType::UpdateMintAuthority { new_authority } => instruction_data
                .with_update_mint_authority(UpdateAuthority {
                    new_authority: new_authority.map(|a| a.to_bytes().into()),
                }),
            MintActionType::UpdateFreezeAuthority { new_authority } => instruction_data
                .with_update_freeze_authority(UpdateAuthority {
                    new_authority: new_authority.map(|a| a.to_bytes().into()),
                }),
            MintActionType::UpdateMetadataField {
                extension_index,
                field_type,
                key,
                value,
            } => instruction_data.with_update_metadata_field(UpdateMetadataFieldAction {
                extension_index,
                field_type,
                key,
                value,
            }),
            MintActionType::UpdateMetadataAuthority {
                extension_index,
                new_authority,
            } => instruction_data.with_update_metadata_authority(UpdateMetadataAuthorityAction {
                extension_index,
                new_authority: new_authority.to_bytes().into(),
            }),
            MintActionType::RemoveMetadataKey {
                extension_index,
                key,
                idempotent,
            } => instruction_data.with_remove_metadata_key(RemoveMetadataKeyAction {
                extension_index,
                key,
                idempotent,
            }),
            MintActionType::DecompressMint {
                cmint_bump,
                rent_payment,
                write_top_up,
            } => instruction_data.with_decompress_mint(DecompressMintAction {
                cmint_bump,
                rent_payment,
                write_top_up,
            }),
            MintActionType::CompressAndCloseCMint { idempotent } => instruction_data
                .with_compress_and_close_cmint(CompressAndCloseCMintAction {
                    idempotent: if idempotent { 1 } else { 0 },
                }),
        };
    }

    // Build account metas configuration
    let mut config = if is_creating_mint {
        MintActionMetaConfig::new_create_mint(
            params.payer,
            params.authority,
            params.mint_seed,
            address_tree_pubkey,
            state_tree_info.queue,
        )
    } else {
        MintActionMetaConfig::new(
            params.payer,
            params.authority,
            state_tree_info.tree,
            state_tree_info.queue,
            state_tree_info.queue,
        )
    };

    // Add tokens_out_queue if there are MintToCompressed actions
    if has_mint_to_compressed {
        config = config.with_mint_compressed_tokens();
    }

    // Add ctoken accounts if any MintToCToken actions were present
    if !ctoken_accounts.is_empty() {
        config = config.with_ctoken_accounts(ctoken_accounts);
    }

    // Add compressible CMint accounts if DecompressMint or CompressAndCloseCMint action is present
    if has_decompress_mint || has_compress_and_close_cmint {
        let (cmint_pda, _) = find_cmint_address(&params.mint_seed);
        // Get config and rent_sponsor from v1 config PDA
        let config_address = CompressibleConfig::ctoken_v1_config_pda();
        let compressible_config: CompressibleConfig = rpc
            .get_anchor_account(&config_address)
            .await?
            .ok_or_else(|| {
                RpcError::CustomError(format!(
                    "CompressibleConfig not found at {}",
                    config_address
                ))
            })?;
        config = config.with_compressible_cmint(
            cmint_pda,
            config_address,
            compressible_config.rent_sponsor,
        );
        // DecompressMint needs mint_signer even when not creating a new mint
        // (for PDA derivation of CMint account)
        // CompressAndCloseCMint does NOT need mint_signer - it verifies CMint via compressed_mint.metadata.mint
        if has_decompress_mint && !is_creating_mint {
            config = config.with_mint_signer(params.mint_seed);
        }
    }

    // Get account metas
    let account_metas = config.to_account_metas();

    // Serialize instruction data
    let data = instruction_data
        .data()
        .map_err(|e| RpcError::CustomError(format!("Failed to serialize instruction: {:?}", e)))?;

    // Build final instruction
    Ok(Instruction {
        program_id: LIGHT_TOKEN_PROGRAM_ID.into(),
        accounts: account_metas,
        data,
    })
}

/// Parameters for decompressing a mint to a CMint Solana account.
/// CMint is always compressible.
#[derive(Debug, Clone)]
pub struct DecompressMintParams {
    /// Rent payment in epochs (prepaid). Must be >= 2.
    pub rent_payment: u8,
    /// Lamports allocated for future write operations (top-up per write).
    pub write_top_up: u32,
}

impl Default for DecompressMintParams {
    fn default() -> Self {
        Self {
            rent_payment: 2, // Minimum valid rent_payment
            write_top_up: 0, // No write top-up by default
        }
    }
}

/// Helper function to create a comprehensive mint action instruction
#[allow(clippy::too_many_arguments)]
pub async fn create_comprehensive_mint_action_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    mint_seed: &Keypair,
    authority: Pubkey,
    payer: Pubkey,
    // Whether to decompress the mint to a CMint Solana account (with rent params)
    decompress_mint: Option<DecompressMintParams>,
    mint_to_recipients: Vec<(Pubkey, u64)>,
    update_mint_authority: Option<Pubkey>,
    update_freeze_authority: Option<Pubkey>,
    // Parameters for mint creation (required when creating a new mint)
    new_mint: Option<NewMint>,
) -> Result<Instruction, RpcError> {
    // Derive addresses
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_cmint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Build actions
    let mut actions = Vec::new();

    if !mint_to_recipients.is_empty() {
        let recipients = mint_to_recipients
            .into_iter()
            .map(|(recipient, amount)| MintToRecipient { recipient, amount })
            .collect();

        actions.push(MintActionType::MintTo {
            recipients,
            token_account_version: 2, // V2 for batched merkle trees
        });
    }

    if let Some(new_authority) = update_mint_authority {
        actions.push(MintActionType::UpdateMintAuthority {
            new_authority: Some(new_authority),
        });
    }

    if let Some(new_authority) = update_freeze_authority {
        actions.push(MintActionType::UpdateFreezeAuthority {
            new_authority: Some(new_authority),
        });
    }

    // Add DecompressMint action if requested
    if let Some(decompress_params) = decompress_mint {
        let (_, cmint_bump) = find_cmint_address(&mint_seed.pubkey());
        actions.push(MintActionType::DecompressMint {
            cmint_bump,
            rent_payment: decompress_params.rent_payment,
            write_top_up: decompress_params.write_top_up,
        });
    }

    create_mint_action_instruction(
        rpc,
        MintActionParams {
            compressed_mint_address,
            mint_seed: mint_seed.pubkey(),
            authority,
            payer,
            actions,
            new_mint,
        },
    )
    .await
}
