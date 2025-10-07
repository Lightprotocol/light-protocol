use light_client::indexer::ValidityProofWithContext;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_account::address::derive_address;
use light_compressed_token_sdk::instructions::create_compressed_mint::{
    create_compressed_mint, find_mint_address, CreateCompressedMintInputs,
};
use light_ctoken_types::{
    instructions::extensions::{
        token_metadata::TokenMetadataInstructionData, ExtensionInstructionData,
    },
    state::TokenDataVersion,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Helper function to create TokenMetadataInstructionData from simple strings
pub fn create_metadata(name: String, symbol: String, uri: String) -> TokenMetadataInstructionData {
    TokenMetadataInstructionData {
        update_authority: None,
        name: name.into_bytes(),
        symbol: symbol.into_bytes(),
        uri: uri.into_bytes(),
        additional_metadata: None,
    }
}

/// Creates a compressed mint instruction with individual parameters.
///
/// # Arguments
/// * `mint_signer` - The signer pubkey used to derive the mint PDA
/// * `mint_authority` - Authority that can mint tokens  
/// * `decimals` - Number of decimal places for the token
/// * `freeze_authority` - Optional authority that can freeze tokens
/// * `validity_proof` - Validity proof with context (contains address tree and proof)
/// * `payer` - Transaction fee payer
/// * `output_queue` - Output queue for the compressed account
/// * `metadata` - Optional token metadata
///
/// # Returns
/// `Result<Instruction, RpcError>` - The compressed mint creation instruction
pub fn create_mint(
    mint_signer: &Pubkey,
    mint_authority: Pubkey,
    decimals: u8,
    freeze_authority: Option<Pubkey>,
    validity_proof: ValidityProofWithContext,
    payer: Pubkey,
    output_queue: Pubkey,
    metadata: Option<TokenMetadataInstructionData>,
) -> Result<Instruction, RpcError> {
    // Derive mint PDA and bump from signer
    let (_, mint_bump) = find_mint_address(mint_signer);

    // Extract address tree from validity proof context
    let address_tree_pubkey = validity_proof
        .addresses
        .first()
        .map(|addr| addr.tree_info.tree)
        .ok_or_else(|| {
            RpcError::CustomError("Missing address tree in validity proof".to_string())
        })?;

    // Extract address root index from context
    let address_root_index = validity_proof
        .get_address_root_indices()
        .get(0)
        .copied()
        .ok_or_else(|| RpcError::CustomError("Missing address root index".to_string()))?;

    // Extract underlying compressed proof
    let compressed_proof = match validity_proof.proof.0 {
        Some(p) => p,
        None => {
            return Err(RpcError::CustomError(
                "Missing compressed proof in ValidityProofWithContext".to_string(),
            ))
        }
    };

    let extensions = metadata.map(|meta| vec![ExtensionInstructionData::TokenMetadata(meta)]);

    let inputs = CreateCompressedMintInputs {
        decimals,
        mint_authority,
        freeze_authority,
        proof: compressed_proof,
        mint_bump,
        address_merkle_tree_root_index: address_root_index,
        mint_signer: *mint_signer,
        payer,
        address_tree_pubkey,
        output_queue,
        extensions,
        version: TokenDataVersion::ShaFlat as u8,
    };

    create_compressed_mint(inputs)
        .map_err(|e| RpcError::CustomError(format!("Token SDK error: {:?}", e)))
}

/// Creates a compressed mint instruction (async). Fetches proof+context and calls create_mint.
///
/// # Arguments
/// * `rpc` - RPC client with indexer capabilities
/// * `mint_seed` - Keypair used to derive the mint PDA
/// * `decimals` - Number of decimal places for the token
/// * `mint_authority` - Authority that can mint tokens
/// * `freeze_authority` - Optional authority that can freeze tokens
/// * `payer` - Fee payer pubkey
/// * `metadata` - Optional metadata for the token
///
/// # Returns
/// `Result<Instruction, RpcError>` - The compressed mint creation instruction
pub async fn create_compressed_mint_instruction_async<R: Rpc + Indexer>(
    rpc: &mut R,
    mint_seed: &Keypair,
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    payer: Pubkey,
    metadata: Option<TokenMetadataInstructionData>,
) -> Result<Instruction, RpcError> {
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let output_queue = rpc.get_random_state_tree_info()?.queue;

    let (mint, _) = find_mint_address(&mint_seed.pubkey());

    let compressed_mint_address = derive_address(
        &mint.to_bytes(),
        &address_tree_pubkey.to_bytes(),
        &light_ctoken_types::CTOKEN_PROGRAM_ID,
    );

    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![light_client::indexer::AddressWithTree {
                address: compressed_mint_address,
                tree: address_tree_pubkey,
            }],
            None,
        )
        .await?
        .value;

    create_mint(
        &mint_seed.pubkey(),
        mint_authority,
        decimals,
        freeze_authority,
        rpc_result,
        payer,
        output_queue,
        metadata,
    )
}
