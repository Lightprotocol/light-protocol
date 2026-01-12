use borsh::BorshDeserialize;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_token_interface::{
    instructions::mint_action::{CompressedMintWithContext, Recipient},
    state::{CompressedMint, TokenDataVersion},
};
use light_ctoken_sdk::compressed_token::{
    create_compressed_mint::derive_cmint_from_spl_mint,
    mint_to_compressed::{
        create_mint_to_compressed_instruction, DecompressedMintConfig, MintToCompressedInputs,
    },
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

/// Creates a mint_to_compressed instruction that mints compressed tokens to recipients
pub async fn mint_to_compressed_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    spl_mint_pda: Pubkey,
    recipients: Vec<Recipient>,
    token_account_version: TokenDataVersion,
    mint_authority: Pubkey,
    payer: Pubkey,
) -> Result<Instruction, RpcError> {
    // Derive compressed mint address from SPL mint PDA
    let address_tree_pubkey = rpc.get_address_tree_v2().tree;
    let compressed_mint_address = derive_cmint_from_spl_mint(&spl_mint_pda, &address_tree_pubkey);

    // Get the compressed mint account
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await?
        .value
        .ok_or(RpcError::AccountDoesNotExist(format!(
            "{:?}",
            compressed_mint_address
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

    // Get state tree info for outputs
    let state_tree_info = rpc.get_random_state_tree_info()?;

    // Check if CMint is decompressed (source of truth)
    let cmint_decompressed = compressed_mint
        .as_ref()
        .map(|m| m.metadata.cmint_decompressed)
        .unwrap_or(true); // If no data, assume CMint is source of truth

    if cmint_decompressed {
        unimplemented!("SPL mint synchronization for decompressed CMint not yet implemented");
    }
    let decompressed_mint_config: Option<DecompressedMintConfig<Pubkey>> = None;
    let spl_interface_pda: Option<light_ctoken_sdk::spl_interface::SplInterfacePda> = None;

    // Prepare compressed mint inputs
    let compressed_mint_inputs = CompressedMintWithContext {
        prove_by_index: rpc_proof_result.accounts[0].root_index.proof_by_index(),
        leaf_index: compressed_mint_account.leaf_index,
        root_index: rpc_proof_result.accounts[0]
            .root_index
            .root_index()
            .unwrap_or_default(),
        address: compressed_mint_address,
        mint: compressed_mint.map(|m| m.try_into().unwrap()),
    };

    // Create the instruction
    create_mint_to_compressed_instruction(
        MintToCompressedInputs {
            cpi_context_pubkey: None,
            compressed_mint_inputs,
            recipients,
            mint_authority,
            payer,
            state_merkle_tree: compressed_mint_account.tree_info.tree,
            input_queue: compressed_mint_account.tree_info.queue,
            output_queue_cmint: compressed_mint_account.tree_info.queue,
            output_queue_tokens: state_tree_info.queue,
            decompressed_mint_config,
            proof: rpc_proof_result.proof.into(),
            token_account_version: token_account_version as u8, // V2 for batched merkle trees
            spl_interface_pda,
        },
        None,
    )
    .map_err(|e| {
        RpcError::CustomError(format!(
            "Failed to create mint_to_compressed instruction: {:?}",
            e
        ))
    })
}
