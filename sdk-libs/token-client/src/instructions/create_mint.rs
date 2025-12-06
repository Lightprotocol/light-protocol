use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_ctoken_interface::instructions::extensions::{
    token_metadata::TokenMetadataInstructionData, ExtensionInstructionData,
};
use light_ctoken_sdk::ctoken::{
    derive_cmint_compressed_address, find_cmint_address, CreateCMint, CreateCMintParams,
};
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// Create a compressed mint instruction with automatic setup.
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
/// `Result<Instruction, TokenSdkError>` - The compressed mint creation instruction
pub async fn create_compressed_mint_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    mint_seed: &Keypair,
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    payer: Pubkey,
    metadata: Option<TokenMetadataInstructionData>,
) -> Result<Instruction, RpcError> {
    // Get address tree and output queue from RPC
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;

    let output_queue = rpc.get_random_state_tree_info()?.queue;

    let compressed_mint_address =
        derive_cmint_compressed_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Create extensions if metadata is provided
    let extensions = metadata.map(|meta| vec![ExtensionInstructionData::TokenMetadata(meta)]);

    // Get validity proof for address creation
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

    let address_merkle_tree_root_index = rpc_result.addresses[0].root_index;

    // Build params struct manually
    let params = CreateCMintParams {
        decimals,
        address_merkle_tree_root_index,
        mint_authority,
        proof: rpc_result.proof.0.unwrap(),
        compression_address: compressed_mint_address,
        mint: find_cmint_address(&mint_seed.pubkey()).0,
        freeze_authority,
        extensions,
    };

    // Create instruction builder
    let builder = CreateCMint::new(
        params,
        mint_seed.pubkey(),
        payer,
        address_tree_pubkey,
        output_queue,
    );

    builder
        .instruction()
        .map_err(|e| RpcError::CustomError(format!("Token SDK error: {:?}", e)))
}
