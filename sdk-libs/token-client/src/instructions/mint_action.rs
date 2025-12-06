use borsh::BorshDeserialize;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_account::instruction_data::traits::LightInstructionData;
use light_ctoken_interface::{
    instructions::{
        extensions::{token_metadata::TokenMetadataInstructionData, ExtensionInstructionData},
        mint_action::{
            CompressedMintWithContext, MintActionCompressedInstructionData, MintToCTokenAction,
            MintToCompressedAction, Recipient, RemoveMetadataKeyAction, UpdateAuthority,
            UpdateMetadataAuthorityAction, UpdateMetadataFieldAction,
        },
    },
    state::CompressedMint,
    COMPRESSED_TOKEN_PROGRAM_ID,
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
}

/// Parameters for creating a new mint
#[derive(Debug)]
pub struct NewMint {
    pub decimals: u8,
    pub supply: u64,
    pub mint_authority: Pubkey,
    pub freeze_authority: Option<Pubkey>,
    pub metadata: Option<TokenMetadataInstructionData>,
    pub version: u8,
}

/// Parameters for mint action instruction
#[derive(Debug)]
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
            light_ctoken_interface::instructions::mint_action::CompressedMintInstructionData {
                supply: new_mint.supply,
                decimals: new_mint.decimals,
                metadata: light_ctoken_interface::state::CompressedMintMetadata {
                    version: new_mint.version,
                    mint: find_cmint_address(&params.mint_seed).0.to_bytes().into(),
                    spl_mint_initialized: false, // Will be set to true if CreateSplMint action is present
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
            mint: mint_data,
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

        // Deserialize the compressed mint
        let compressed_mint: CompressedMint = BorshDeserialize::deserialize(
            &mut compressed_mint_account.data.unwrap().data.as_slice(),
        )
        .map_err(|e| {
            RpcError::CustomError(format!("Failed to deserialize compressed mint: {}", e))
        })?;

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
            mint: compressed_mint.try_into().unwrap(),
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
            params.compressed_mint_address,
            compressed_mint_inputs.root_index,
            proof.ok_or_else(|| {
                RpcError::CustomError("Proof is required for mint creation".to_string())
            })?,
            compressed_mint_inputs.mint.clone(),
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

                instruction_data.with_mint_to_ctoken(MintToCTokenAction {
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

    // Get account metas
    let account_metas = config.to_account_metas();

    // Serialize instruction data
    let data = instruction_data
        .data()
        .map_err(|e| RpcError::CustomError(format!("Failed to serialize instruction: {:?}", e)))?;

    // Build final instruction
    Ok(Instruction {
        program_id: COMPRESSED_TOKEN_PROGRAM_ID.into(),
        accounts: account_metas,
        data,
    })
}

/// Helper function to create a comprehensive mint action instruction
#[allow(clippy::too_many_arguments)]
pub async fn create_comprehensive_mint_action_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    mint_seed: &Keypair,
    authority: Pubkey,
    payer: Pubkey,
    create_spl_mint: bool,
    mint_to_recipients: Vec<(Pubkey, u64)>,
    update_mint_authority: Option<Pubkey>,
    update_freeze_authority: Option<Pubkey>,
    // Parameters for mint creation (required if create_spl_mint is true)
    new_mint: Option<NewMint>,
) -> Result<Instruction, RpcError> {
    // Derive addresses
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address =
        derive_cmint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Build actions
    let mut actions = Vec::new();

    if create_spl_mint {
        return Err(RpcError::CustomError(
            "CreateSplMint is no longer supported".to_string(),
        ));
    }

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
