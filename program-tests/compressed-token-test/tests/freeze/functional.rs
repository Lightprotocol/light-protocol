//! Tests for freezing/thawing compressed token accounts with different TokenDataVersions (no TLV).
//!
//! Verifies that compressed token accounts can be frozen/thawed using different
//! hashing versions without TLV extensions, and then decompressed.

use light_client::indexer::{CompressedTokenAccount, Indexer};
use light_compressed_token::freeze::sdk::{
    create_instruction, CreateInstructionInputs as FreezeInputs,
};
use light_program_test::{LightProgramTest, ProgramTestConfig};
use light_prover_client::prover::spawn_prover;
use light_test_utils::{
    actions::transfer2::{compress_with_version, decompress},
    conversions::sdk_to_program_token_data,
    mint_2022::create_token_22_account,
    spl::create_mint_22_helper,
    Rpc, RpcError,
};
use light_token::compat::{AccountState, TokenDataWithMerkleContext};
use light_token_interface::state::TokenDataVersion;
use serial_test::serial;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, signature::Keypair, signer::Signer};

/// Helper to append version byte to the inner inputs Vec of an Anchor instruction.
/// Anchor instruction format: [8 bytes discriminator][4 bytes Vec length][N bytes Vec content]
fn append_version_to_inputs(instruction: &mut solana_sdk::instruction::Instruction, version: u8) {
    // The Vec length is at bytes 8..12 (little endian u32)
    let len_bytes = &instruction.data[8..12];
    let current_len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]);

    // Increment the length
    let new_len = current_len + 1;
    instruction.data[8..12].copy_from_slice(&new_len.to_le_bytes());

    // Append the version byte to the data
    instruction.data.push(version);
}

/// Helper to create and send freeze or thaw instruction with specified version
async fn freeze_or_thaw<const FREEZE: bool>(
    rpc: &mut LightProgramTest,
    payer: &Keypair,
    compressed_accounts: Vec<TokenDataWithMerkleContext>,
    output_merkle_tree: &Pubkey,
    version: TokenDataVersion,
) -> Result<(), RpcError> {
    // Get validity proofs for the compressed accounts
    let input_compressed_account_hashes = compressed_accounts
        .iter()
        .map(|x| x.compressed_account.hash().unwrap())
        .collect::<Vec<_>>();

    let proof_rpc_result = rpc
        .get_validity_proof(input_compressed_account_hashes.clone(), vec![], None)
        .await?;

    let inputs = FreezeInputs {
        fee_payer: payer.pubkey(),
        authority: payer.pubkey(),
        input_merkle_contexts: compressed_accounts
            .iter()
            .map(|x| x.compressed_account.merkle_context)
            .collect(),
        input_token_data: compressed_accounts
            .iter()
            .cloned()
            .map(|x| x.token_data)
            .map(sdk_to_program_token_data)
            .collect(),
        input_compressed_accounts: compressed_accounts
            .iter()
            .map(|x| x.compressed_account.compressed_account.clone())
            .collect::<Vec<_>>(),
        outputs_merkle_tree: *output_merkle_tree,
        root_indices: proof_rpc_result
            .value
            .accounts
            .iter()
            .map(|x| x.root_index.root_index())
            .collect::<Vec<_>>(),
        proof: proof_rpc_result.value.proof.0.unwrap_or_default(),
    };

    let mut instruction = create_instruction::<FREEZE>(inputs).map_err(|e| {
        RpcError::CustomError(format!("Failed to create freeze instruction: {:?}", e))
    })?;

    // Append version byte to the inner inputs Vec
    append_version_to_inputs(&mut instruction, version as u8);

    rpc.create_and_send_transaction(&[instruction], &payer.pubkey(), &[payer])
        .await?;

    Ok(())
}

/// Test that compressed token accounts can be frozen/thawed with V1 (no TLV)
/// and then decompressed.
#[tokio::test]
#[serial]
async fn test_freeze_thaw_v1_no_tlv_and_decompress() {
    spawn_prover().await;
    let result = run_freeze_thaw_test(TokenDataVersion::V1).await;
    assert!(result.is_ok(), "Test failed: {:?}", result.err());
}

/// Test that compressed token accounts can be frozen/thawed with V2 (no TLV)
/// and then decompressed.
#[tokio::test]
#[serial]
async fn test_freeze_thaw_v2_no_tlv_and_decompress() {
    spawn_prover().await;
    let result = run_freeze_thaw_test(TokenDataVersion::V2).await;
    assert!(result.is_ok(), "Test failed: {:?}", result.err());
}

/// Test that compressed token accounts can be frozen/thawed with ShaFlat (no TLV)
/// and then decompressed.
#[tokio::test]
#[serial]
async fn test_freeze_thaw_sha_flat_no_tlv_and_decompress() {
    spawn_prover().await;
    let result = run_freeze_thaw_test(TokenDataVersion::ShaFlat).await;
    assert!(result.is_ok(), "Test failed: {:?}", result.err());
}

/// Parameterized test for freeze/thaw with specified TokenDataVersion.
///
/// Flow:
/// 1. Create mint without extensions (payer is freeze authority)
/// 2. Create SPL token account and mint tokens
/// 3. Compress tokens
/// 4. Freeze the compressed token account using specified version
/// 5. Verify frozen state
/// 6. Thaw the compressed token account using specified version
/// 7. Verify thawed state
/// 8. Decompress tokens back to SPL
/// 9. Verify decompression succeeded
async fn run_freeze_thaw_test(version: TokenDataVersion) -> Result<(), RpcError> {
    let mut rpc = LightProgramTest::new(ProgramTestConfig::new_v2(false, None)).await?;
    let payer = rpc.get_payer().insecure_clone();
    let env = rpc.test_accounts.clone();

    // 1. Create Token-2022 mint without extensions (payer is freeze authority)
    let mint_pubkey = create_mint_22_helper(&mut rpc, &payer).await;

    // 2. Create SPL Token-2022 account and mint tokens
    let spl_account =
        create_token_22_account(&mut rpc, &payer, &mint_pubkey, &payer.pubkey()).await;
    let mint_amount = 1_000_000u64;
    light_test_utils::mint_2022::mint_spl_tokens_22(
        &mut rpc,
        &payer,
        &mint_pubkey,
        &spl_account,
        mint_amount,
    )
    .await;

    // Get output merkle tree
    let output_merkle_tree: Pubkey = env.v2_state_trees[0].output_queue;

    // 3. Compress tokens using transfer2 with specified version
    compress_with_version(
        &mut rpc,
        spl_account,
        mint_amount,
        payer.pubkey(),
        &payer,
        &payer,
        2, // decimals (CREATE_MINT_HELPER_DECIMALS)
        version,
    )
    .await
    .map_err(|e| RpcError::CustomError(format!("Failed to compress: {:?}", e)))?;

    // 4. Get compressed accounts and verify initial state
    let compressed_accounts: Vec<TokenDataWithMerkleContext> = rpc
        .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
        .await?
        .into();

    assert_eq!(
        compressed_accounts.len(),
        1,
        "Should have exactly 1 compressed token account"
    );
    assert_eq!(
        compressed_accounts[0].token_data.state,
        AccountState::Initialized,
        "Initial state should be Initialized"
    );
    assert!(
        compressed_accounts[0].token_data.tlv.is_none(),
        "Token account should have no TLV"
    );

    // 5. Freeze the compressed token account
    freeze_or_thaw::<true>(
        &mut rpc,
        &payer,
        compressed_accounts.clone(),
        &output_merkle_tree,
        version,
    )
    .await?;

    // 6. Verify frozen state
    let frozen_accounts: Vec<TokenDataWithMerkleContext> = rpc
        .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
        .await?
        .into();

    assert_eq!(
        frozen_accounts.len(),
        1,
        "Should still have exactly 1 compressed token account"
    );
    assert_eq!(
        frozen_accounts[0].token_data.state,
        AccountState::Frozen,
        "Token account should be frozen"
    );

    // 7. Thaw the compressed token account
    freeze_or_thaw::<false>(
        &mut rpc,
        &payer,
        frozen_accounts.clone(),
        &output_merkle_tree,
        version,
    )
    .await?;

    // 8. Verify thawed state
    let thawed_accounts: Vec<TokenDataWithMerkleContext> = rpc
        .get_compressed_token_accounts_by_owner(&payer.pubkey(), None, None)
        .await?
        .into();

    assert_eq!(
        thawed_accounts.len(),
        1,
        "Should still have exactly 1 compressed token account"
    );
    assert_eq!(
        thawed_accounts[0].token_data.state,
        AccountState::Initialized,
        "Token account should be thawed (Initialized)"
    );

    // 9. Decompress tokens back to SPL
    let compressed_accounts: Vec<CompressedTokenAccount> = thawed_accounts
        .into_iter()
        .map(|a| a.try_into().unwrap())
        .collect();
    decompress(
        &mut rpc,
        &compressed_accounts,
        mint_amount,
        spl_account,
        &payer,
        &payer,
        2, // decimals
    )
    .await
    .map_err(|e| RpcError::CustomError(format!("Failed to decompress: {:?}", e)))?;

    // 10. Verify SPL token account balance
    let token_account_data = rpc.get_account(spl_account).await?.unwrap();
    let token_account =
        spl_token_2022::state::Account::unpack(&token_account_data.data[..165]).unwrap();
    assert_eq!(
        token_account.amount, mint_amount,
        "SPL token account should have full balance after decompress"
    );

    println!(
        "Successfully froze/thawed with {:?} (no TLV) and decompressed",
        version
    );

    Ok(())
}
