use borsh::BorshDeserialize;
use light_client::{
    indexer::Indexer,
    rpc::{Rpc, RpcError},
};
use light_compressed_account::instruction_data::traits::LightInstructionData;
use light_compressed_token_sdk::compressed_token::{
    create_compressed_mint::find_mint_address, mint_action::MintActionMetaConfig,
};
use light_compressible::config::CompressibleConfig;
use light_token_interface::{
    instructions::mint_action::{
        CompressAndCloseMintAction, MintActionCompressedInstructionData, MintWithContext,
    },
    state::Mint,
    LIGHT_TOKEN_PROGRAM_ID,
};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

/// Creates a CompressAndCloseMint instruction by fetching required data from RPC/indexer.
///
/// This is permissionless - anyone can call when the mint is compressible (rent expired).
///
/// # Parameters
/// - `rpc`: RPC client that also implements Indexer
/// - `payer`: Account paying for transaction fees
/// - `compressed_mint_address`: The 32-byte compressed address of the mint
/// - `mint_seed`: The seed pubkey used to derive the mint PDA
/// - `idempotent`: If true, succeed silently when Mint doesn't exist
pub async fn create_compress_and_close_mint_instruction<R: Rpc + Indexer>(
    rpc: &mut R,
    payer: Pubkey,
    compressed_mint_address: [u8; 32],
    mint_seed: Pubkey,
    idempotent: bool,
) -> Result<Instruction, RpcError> {
    // Derive the mint PDA from mint_seed
    let (mint_pda, _bump) = find_mint_address(&mint_seed);

    // Get the compressed mint account
    let compressed_mint_account = rpc
        .get_compressed_account(compressed_mint_address, None)
        .await?
        .value
        .ok_or_else(|| RpcError::AccountDoesNotExist(format!("{:?}", compressed_mint_address)))?;

    // Try to deserialize the compressed mint - may be None if Mint is already decompressed
    let compressed_mint: Option<Mint> = compressed_mint_account
        .data
        .as_ref()
        .and_then(|d| BorshDeserialize::deserialize(&mut d.data.as_slice()).ok());

    // Get validity proof for the compressed mint
    let rpc_proof_result = rpc
        .get_validity_proof(vec![compressed_mint_account.hash], vec![], None)
        .await?
        .value;

    // Build MintWithContext
    let compressed_mint_inputs = MintWithContext {
        prove_by_index: rpc_proof_result.accounts[0].root_index.proof_by_index(),
        leaf_index: compressed_mint_account.leaf_index,
        root_index: rpc_proof_result.accounts[0]
            .root_index
            .root_index()
            .unwrap_or_default(),
        address: compressed_mint_address,
        mint: compressed_mint.map(|m| m.try_into().unwrap()),
    };

    // Build instruction data with CompressAndCloseMint action
    let instruction_data = MintActionCompressedInstructionData::new(
        compressed_mint_inputs,
        rpc_proof_result.proof.into(),
    )
    .with_compress_and_close_mint(CompressAndCloseMintAction {
        idempotent: if idempotent { 1 } else { 0 },
    });

    // Get CompressibleConfig for rent_sponsor
    let config_address = CompressibleConfig::light_token_v1_config_pda();
    let compressible_config: CompressibleConfig = rpc
        .get_anchor_account(&config_address)
        .await?
        .ok_or_else(|| {
            RpcError::CustomError(format!(
                "CompressibleConfig not found at {}",
                config_address
            ))
        })?;

    // Build account metas configuration
    let state_tree_info = rpc_proof_result.accounts[0].tree_info;
    let config = MintActionMetaConfig::new(
        payer,
        payer, // authority - permissionless, using payer
        state_tree_info.tree,
        state_tree_info.queue,
        state_tree_info.queue,
    )
    .with_compressible_mint(mint_pda, config_address, compressible_config.rent_sponsor);

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
