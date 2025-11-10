use borsh::BorshDeserialize;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::instructions::{
    create_mint_action, derive_compressed_mint_address, derive_token_pool, find_spl_mint_address,
    mint_action::{MintActionInputs, MintActionType, MintToRecipient},
};
use light_ctoken_types::{
    instructions::{
        extensions::{token_metadata::TokenMetadataInstructionData, ExtensionInstructionData},
        mint_action::CompressedMintWithContext,
    },
    state::CompressedMint,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

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
    /// Required if any action is CreateSplMint
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
            light_ctoken_types::instructions::mint_action::CompressedMintInstructionData {
                supply: new_mint.supply,
                decimals: new_mint.decimals,
                metadata: light_ctoken_types::state::CompressedMintMetadata {
                    version: new_mint.version,
                    mint: find_spl_mint_address(&params.mint_seed).0.to_bytes().into(),
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
    println!("compressed_mint_inputs {:?}", compressed_mint_inputs);
    // Get mint bump from find_spl_mint_address if we're creating a compressed mint
    let mint_bump = if is_creating_mint {
        Some(find_spl_mint_address(&params.mint_seed).1)
    } else {
        None
    };

    // Check if we need token_pool (for SPL operations)
    let needs_token_pool = params.actions.iter().any(|action| {
        matches!(
            action,
            MintActionType::CreateSplMint { .. } | MintActionType::MintToCToken { .. }
        )
    }) || compressed_mint_inputs.mint.metadata.spl_mint_initialized;

    let token_pool = if needs_token_pool {
        let mint = find_spl_mint_address(&params.mint_seed).0;
        Some(derive_token_pool(&mint, 0))
    } else {
        None
    };

    // Create the mint action instruction inputs
    let instruction_inputs = MintActionInputs {
        compressed_mint_inputs,
        mint_seed: params.mint_seed,
        create_mint: is_creating_mint,
        mint_bump,
        authority: params.authority,
        payer: params.payer,
        proof,
        actions: params.actions,
        // address_tree when create_mint, input state tree when not
        address_tree_pubkey: if is_creating_mint {
            address_tree_pubkey
        } else {
            state_tree_info.tree
        },
        // input_queue only needed when operating on existing mint
        input_queue: if is_creating_mint {
            None
        } else {
            Some(state_tree_info.queue)
        },
        output_queue: state_tree_info.queue,
        tokens_out_queue: Some(state_tree_info.queue), // Output queue for tokens
        token_pool,
    };

    // Create the instruction using the SDK
    let instruction = create_mint_action(instruction_inputs).map_err(|e| {
        RpcError::CustomError(format!("Failed to create mint action instruction: {:?}", e))
    })?;

    Ok(instruction)
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
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);
    let (_, mint_bump) = find_spl_mint_address(&mint_seed.pubkey());

    // Build actions
    let mut actions = Vec::new();

    if create_spl_mint {
        actions.push(MintActionType::CreateSplMint { mint_bump });
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
