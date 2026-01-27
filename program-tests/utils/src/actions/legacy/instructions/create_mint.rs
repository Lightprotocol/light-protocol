use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_token_interface::instructions::extensions::token_metadata::TokenMetadataInstructionData;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

use super::mint_action::{create_mint_action_instruction, MintActionParams, NewMint};

/// Create a compressed-only mint instruction (no decompression).
///
/// This creates ONLY the compressed mint account, NOT the Mint Solana account.
/// Use DecompressMint action to create the Mint Solana account later if needed.
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
pub async fn create_compressed_mint_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    mint_seed: &Keypair,
    decimals: u8,
    mint_authority: Pubkey,
    freeze_authority: Option<Pubkey>,
    payer: Pubkey,
    metadata: Option<TokenMetadataInstructionData>,
) -> Result<Instruction, RpcError> {
    // Get address tree for deriving compressed mint address
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address = light_token::instruction::derive_mint_compressed_address(
        &mint_seed.pubkey(),
        &address_tree_pubkey,
    );

    // Create compressed-only mint using MintAction with empty actions
    create_mint_action_instruction(
        rpc,
        MintActionParams {
            compressed_mint_address,
            mint_seed: mint_seed.pubkey(),
            authority: mint_authority,
            payer,
            actions: vec![], // No actions - just create compressed mint
            new_mint: Some(NewMint {
                decimals,
                supply: 0,
                mint_authority,
                freeze_authority,
                metadata,
                version: 3,
            }),
        },
    )
    .await
}
