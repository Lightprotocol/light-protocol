use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_token_sdk::instructions::create_compressed_mint::{
    create_compressed_mint, derive_compressed_mint_address, CreateCompressedMintInputs,
};
use light_ctoken_types::{
    instructions::extensions::{
        token_metadata::TokenMetadataInstructionData, ExtensionInstructionData,
    },
    COMPRESSED_MINT_SEED,
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
        derive_compressed_mint_address(&mint_seed.pubkey(), &address_tree_pubkey);

    // Find mint bump for the instruction
    let (_, mint_bump) = Pubkey::find_program_address(
        &[COMPRESSED_MINT_SEED, mint_seed.pubkey().as_ref()],
        &Pubkey::new_from_array(light_ctoken_types::COMPRESSED_TOKEN_PROGRAM_ID),
    );

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

    // Create instruction using the existing SDK function
    let inputs = CreateCompressedMintInputs {
        decimals,
        mint_authority,
        freeze_authority,
        proof: rpc_result.proof.0.unwrap(),
        mint_bump,
        address_merkle_tree_root_index,
        mint_signer: mint_seed.pubkey(),
        payer,
        address_tree_pubkey,
        output_queue,
        extensions,
        version: 3,
    };

    create_compressed_mint(inputs)
        .map_err(|e| RpcError::CustomError(format!("Token SDK error: {:?}", e)))
}
