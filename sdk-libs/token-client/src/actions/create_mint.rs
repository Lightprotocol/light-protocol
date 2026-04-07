//! Create mint action for Light Token.
//!
//! This action provides a clean interface for creating a new Light Token mint.

use light_client::{
    indexer::{AddressWithTree, Indexer},
    rpc::{Rpc, RpcError},
};
use light_token::instruction::{
    derive_mint_compressed_address, find_mint_address, CreateMint as CreateMintInstruction,
    CreateMintParams as CreateMintInstructionParams,
};
use light_token_interface::{
    instructions::extensions::{ExtensionInstructionData, TokenMetadataInstructionData},
    state::AdditionalMetadata,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_signer::Signer;

/// Token metadata for the mint.
#[derive(Clone, Debug, Default)]
pub struct TokenMetadata {
    /// The longer name of the token.
    pub name: String,
    /// The shortened symbol for the token.
    pub symbol: String,
    /// The URI pointing to richer metadata.
    pub uri: String,
    /// Authority that can update the metadata.
    pub update_authority: Option<Pubkey>,
    /// Additional metadata as key-value pairs.
    pub additional_metadata: Option<Vec<(String, String)>>,
}

/// Parameters for creating a new Light Token mint.
///
/// This creates both a compressed mint AND a decompressed Mint Solana account
/// in a single instruction.
///
/// # Example
/// ```ignore
/// let (signature, mint) = CreateMint {
///     decimals: 9,
///     freeze_authority: Some(freeze_authority_pubkey),
///     token_metadata: Some(TokenMetadata {
///         name: "My Token".to_string(),
///         symbol: "MTK".to_string(),
///         uri: "https://example.com/metadata.json".to_string(),
///         ..Default::default()
///     }),
///     seed: None, // auto-generate, or Some(keypair) for deterministic address
/// }.execute(&mut rpc, &payer, &mint_authority).await?;
/// ```
#[derive(Default, Debug)]
pub struct CreateMint {
    /// Number of decimals for the token.
    pub decimals: u8,
    /// Optional authority that can freeze token accounts.
    pub freeze_authority: Option<Pubkey>,
    /// Optional token metadata (name, symbol, uri).
    pub token_metadata: Option<TokenMetadata>,
    /// Optional seed keypair for deterministic mint address.
    /// If None, a new keypair is generated.
    pub seed: Option<Keypair>,
}

pub struct CreateMintInstructions {
    pub instructions: Vec<Instruction>,
    pub mint: Pubkey,
    pub mint_seed: Keypair,
}

pub async fn create_mint_instructions<R: Rpc + Indexer>(
    rpc: &R,
    create_mint: CreateMint,
    payer: Pubkey,
    mint_authority: Pubkey,
) -> Result<CreateMintInstructions, RpcError> {
    let mint_seed = create_mint.seed.unwrap_or_else(Keypair::new);
    let address_tree = rpc.get_address_tree_v2();
    let output_queue = rpc.get_random_state_tree_info()?.queue;

    // Derive compression address
    let compression_address =
        derive_mint_compressed_address(&mint_seed.pubkey(), &address_tree.tree);

    // Find mint PDA
    let (mint, bump) = find_mint_address(&mint_seed.pubkey());

    // Get validity proof for the address
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: compression_address,
                tree: address_tree.tree,
            }],
            None,
        )
        .await
        .map_err(|e| RpcError::CustomError(format!("Failed to get validity proof: {}", e)))?
        .value;

    // Build extensions if token metadata is provided
    let extensions = create_mint.token_metadata.map(|metadata| {
        let additional_metadata = metadata.additional_metadata.map(|items| {
            items
                .into_iter()
                .map(|(key, value)| AdditionalMetadata {
                    key: key.into_bytes(),
                    value: value.into_bytes(),
                })
                .collect()
        });

        vec![ExtensionInstructionData::TokenMetadata(
            TokenMetadataInstructionData {
                update_authority: Some(
                    metadata
                        .update_authority
                        .unwrap_or(mint_authority)
                        .to_bytes()
                        .into(),
                ),
                name: metadata.name.into_bytes(),
                symbol: metadata.symbol.into_bytes(),
                uri: metadata.uri.into_bytes(),
                additional_metadata,
            },
        )]
    });

    // Build params
    let params = CreateMintInstructionParams {
        decimals: create_mint.decimals,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority,
        proof: rpc_result.proof.0.ok_or_else(|| {
            RpcError::CustomError("Validity proof is required for create_mint".to_string())
        })?,
        compression_address,
        mint,
        bump,
        freeze_authority: create_mint.freeze_authority,
        extensions,
        rent_payment: 16,  // ~24 hours rent
        write_top_up: 766, // ~3 hours per write
    };

    // Create instruction
    let instruction = CreateMintInstruction::new(
        params,
        mint_seed.pubkey(),
        payer,
        address_tree.tree,
        output_queue,
    )
    .instruction()
    .map_err(|e| RpcError::CustomError(format!("Failed to create instruction: {}", e)))?;

    Ok(CreateMintInstructions {
        instructions: vec![instruction],
        mint,
        mint_seed,
    })
}

impl CreateMint {
    pub async fn instructions<R: Rpc + Indexer>(
        self,
        rpc: &R,
        payer: Pubkey,
        mint_authority: Pubkey,
    ) -> Result<CreateMintInstructions, RpcError> {
        create_mint_instructions(rpc, self, payer, mint_authority).await
    }

    /// Execute the create_mint action via RPC.
    ///
    /// # Arguments
    /// * `rpc` - RPC client that implements both `Rpc` and `Indexer` traits
    /// * `payer` - Transaction fee payer keypair
    /// * `mint_authority` - Authority that can mint new tokens
    ///
    /// # Returns
    /// `Result<(Signature, Pubkey), RpcError>` - The transaction signature and mint public key
    pub async fn execute<R: Rpc + Indexer>(
        self,
        rpc: &mut R,
        payer: &Keypair,
        mint_authority: &Keypair,
    ) -> Result<(Signature, Pubkey), RpcError> {
        let instruction_bundle =
            create_mint_instructions(rpc, self, payer.pubkey(), mint_authority.pubkey()).await?;

        // Build signers list
        let mut signers: Vec<&Keypair> = vec![payer, &instruction_bundle.mint_seed];
        if mint_authority.pubkey() != payer.pubkey() {
            signers.push(mint_authority);
        }

        // Send transaction
        let signature = rpc
            .create_and_send_transaction(
                &instruction_bundle.instructions,
                &payer.pubkey(),
                &signers,
            )
            .await?;

        Ok((signature, instruction_bundle.mint))
    }
}
