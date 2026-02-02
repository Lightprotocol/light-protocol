/// Test scenarios for indexer interface endpoints.
///
/// This test creates various account types for testing the indexer's interface racing logic.
/// After running, use `cargo xtask export-photon-test-data --test-name indexer_interface`
/// to export transactions to the indexer's test snapshot directory.
///
/// Scenarios covered:
/// 1. SPL Mint (on-chain) - standard mint for token operations
/// 2. Compressed token accounts (via mint_to) - for getTokenAccountInterface
/// 3. Registered v2 address in batched address tree - for address tree verification
/// 4. Decompressed mint (via CreateMint with rent_payment=0) - for getMintInterface (on-chain CMint)
/// 5. Fully compressed mint (CreateMint + CompressAndCloseMint) - for getMintInterface (compressed DB)
/// 6. Compressible token accounts - on-chain accounts that can be compressed
use std::{collections::HashMap, time::Duration};

use anchor_lang::Discriminator;
use borsh::BorshSerialize;
use create_address_test_program::create_invoke_cpi_instruction;
use forester_utils::utils::wait_for_indexer;
use light_client::{
    indexer::{photon_indexer::PhotonIndexer, AddressWithTree, ColdContext, Indexer},
    local_test_validator::{spawn_validator, LightValidatorConfig},
    rpc::{LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::{
    address::derive_address,
    instruction_data::{
        data::NewAddressParamsAssigned, with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
};
use light_compressed_token::{
    process_mint::mint_sdk::create_mint_to_instruction,
    process_transfer::transfer_sdk::to_account_metas,
};
use light_test_utils::{
    actions::legacy::{
        create_compressible_token_account,
        instructions::mint_action::{
            create_mint_action_instruction, MintActionParams, MintActionType,
        },
        CreateCompressibleTokenAccountInputs,
    },
    pack::pack_new_address_params_assigned,
    spl::create_mint_helper_with_keypair,
};
use light_token::instruction::{
    derive_mint_compressed_address, find_mint_address, CreateMint, CreateMintParams,
};
use light_token_interface::state::TokenDataVersion;
use serial_test::serial;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use tokio::time::sleep;

const COMPUTE_BUDGET_LIMIT: u32 = 1_000_000;

/// Helper to mint compressed tokens
async fn mint_compressed_tokens<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    mint_pubkey: &Pubkey,
    recipients: Vec<Pubkey>,
    amounts: Vec<u64>,
) -> Signature {
    let mint_to_ix = create_mint_to_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        mint_pubkey,
        merkle_tree_pubkey,
        amounts,
        recipients,
        None,
        false,
        0,
    );
    let instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
        mint_to_ix,
    ];
    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .unwrap()
}

/// Test that creates scenarios for Photon interface testing
///
/// Run with: cargo test -p forester --test test_indexer_interface -- --nocapture
/// Then export: cargo xtask export-photon-test-data --test-name indexer_interface
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn test_indexer_interface_scenarios() {
    // Start validator with indexer, prover, and create_address_test_program
    spawn_validator(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        wait_time: 90,
        sbf_programs: vec![(
            "FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy".to_string(),
            "../target/deploy/create_address_test_program.so".to_string(),
        )],
        upgradeable_programs: vec![],
        limit_ledger_size: None,
        validator_args: vec![],
        use_surfpool: true,
    })
    .await;

    let mut rpc = LightClient::new(LightClientConfig::local())
        .await
        .expect("Failed to create LightClient");
    rpc.get_latest_active_state_trees()
        .await
        .expect("Failed to get state trees");

    let payer = rpc.get_payer().insecure_clone();
    rpc.airdrop_lamports(&payer.pubkey(), 100_000_000_000)
        .await
        .expect("Failed to airdrop to payer");

    // Give extra time for indexer to fully start
    sleep(Duration::from_secs(5)).await;

    // Wait for indexer to be ready before making any requests
    wait_for_indexer(&rpc)
        .await
        .expect("Failed to wait for indexer");

    println!("\n========== PHOTON INTERFACE TEST ==========\n");
    println!("Payer: {}", payer.pubkey());

    // ============ Scenario 1: Create SPL Mint ============
    println!("\n=== Creating SPL mint ===");

    let mint_keypair = Keypair::new();
    let mint_pubkey = create_mint_helper_with_keypair(&mut rpc, &payer, &mint_keypair).await;
    println!("SPL Mint: {}", mint_pubkey);

    // ============ Scenario 2: Create compressed token accounts ============
    println!("\n=== Creating compressed token accounts ===");

    let bob = Keypair::new();
    let charlie = Keypair::new();

    let state_tree_info = rpc.get_random_state_tree_info().unwrap();

    // Mint compressed tokens to Bob and Charlie
    let mint_sig = mint_compressed_tokens(
        &mut rpc,
        &state_tree_info.queue,
        &payer,
        &mint_pubkey,
        vec![bob.pubkey(), charlie.pubkey()],
        vec![1_000_000_000, 500_000_000],
    )
    .await;
    println!("Minted compressed tokens: {}", mint_sig);
    println!("Bob pubkey: {}", bob.pubkey());
    println!("Charlie pubkey: {}", charlie.pubkey());

    // Wait for indexer
    sleep(Duration::from_secs(3)).await;

    // ============ Scenario 3: Register v2 Address (using create_address_test_program) ============
    println!("\n=== Registering v2 address in batched address tree ===");

    // Use v2 (batched) address tree
    let address_tree = rpc.get_address_tree_v2();

    // Create a deterministic seed for the address
    let address_seed: [u8; 32] = [42u8; 32];

    // Derive address using v2 method (includes program ID)
    let derived_address = derive_address(
        &address_seed,
        &address_tree.tree.to_bytes(),
        &create_address_test_program::ID.to_bytes(),
    );

    println!("Derived v2 address: {:?}", derived_address);

    // Get validity proof for the new address
    wait_for_indexer(&rpc).await.unwrap();
    let proof_result = rpc
        .indexer()
        .unwrap()
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: derived_address,
                tree: address_tree.tree,
            }],
            None,
        )
        .await
        .unwrap();

    // Build new address params
    let new_address_params = vec![NewAddressParamsAssigned {
        seed: address_seed,
        address_queue_pubkey: address_tree.tree.into(), // For batched trees, queue = tree
        address_merkle_tree_pubkey: address_tree.tree.into(),
        address_merkle_tree_root_index: proof_result.value.get_address_root_indices()[0],
        assigned_account_index: None,
    }];

    // Pack the address params for the instruction
    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
    let packed_new_address_params =
        pack_new_address_params_assigned(&new_address_params, &mut remaining_accounts);

    // Build instruction data for create_address_test_program
    let ix_data = InstructionDataInvokeCpiWithReadOnly {
        mode: 0,
        bump: 255,
        with_cpi_context: false,
        invoking_program_id: create_address_test_program::ID.into(),
        proof: proof_result.value.proof.0,
        new_address_params: packed_new_address_params,
        is_compress: false,
        compress_or_decompress_lamports: 0,
        output_compressed_accounts: Default::default(),
        input_compressed_accounts: Default::default(),
        with_transaction_hash: true,
        read_only_accounts: Vec::new(),
        read_only_addresses: Vec::new(),
        cpi_context: Default::default(),
    };

    let remaining_accounts_metas = to_account_metas(remaining_accounts);

    // Create the instruction using the test program
    let instruction = create_invoke_cpi_instruction(
        payer.pubkey(),
        [
            light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR.to_vec(),
            ix_data.try_to_vec().unwrap(),
        ]
        .concat(),
        remaining_accounts_metas,
        None,
    );

    let instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
        instruction,
    ];
    let address_sig = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[&payer])
        .await
        .unwrap();
    println!(
        "Registered v2 address: {} (sig: {})",
        hex::encode(derived_address),
        address_sig
    );

    // ============ Scenario 4: Decompressed Mint (CreateMint with rent_payment=0) ============
    // This creates a compressed mint that is immediately decompressed to an on-chain CMint account.
    // The compressed account only contains the 32-byte mint_pda reference (DECOMPRESSED_PDA_DISCRIMINATOR).
    // Full mint data is on-chain in the CMint account owned by LIGHT_TOKEN_PROGRAM_ID.
    println!("\n=== Creating decompressed mint (on-chain CMint) ===");

    let decompressed_mint_seed = Keypair::new();
    let output_queue = rpc.get_random_state_tree_info().unwrap().queue;

    // Use v2 address tree for compressed mints
    let mint_address_tree = rpc.get_address_tree_v2();

    // Derive compression address for decompressed mint
    let decompressed_mint_compression_address =
        derive_mint_compressed_address(&decompressed_mint_seed.pubkey(), &mint_address_tree.tree);

    let (decompressed_mint_pda, decompressed_mint_bump) =
        find_mint_address(&decompressed_mint_seed.pubkey());

    // Get validity proof for the address
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: decompressed_mint_compression_address,
                tree: mint_address_tree.tree,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    // Create decompressed mint (CreateMint always creates both compressed + on-chain CMint)
    let decompressed_mint_params = CreateMintParams {
        decimals: 6,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority: payer.pubkey(),
        proof: rpc_result.proof.0.unwrap(),
        compression_address: decompressed_mint_compression_address,
        mint: decompressed_mint_pda,
        bump: decompressed_mint_bump,
        freeze_authority: None,
        extensions: None,
        rent_payment: 0, // Immediately compressible
        write_top_up: 0,
    };

    let create_decompressed_mint_builder = CreateMint::new(
        decompressed_mint_params,
        decompressed_mint_seed.pubkey(),
        payer.pubkey(),
        mint_address_tree.tree,
        output_queue,
    );
    let ix = create_decompressed_mint_builder.instruction().unwrap();

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer, &decompressed_mint_seed],
        blockhash,
    );
    let decompressed_mint_sig = rpc.process_transaction(tx).await.unwrap();
    println!(
        "Created decompressed mint (CMint on-chain): {} (sig: {})",
        decompressed_mint_pda, decompressed_mint_sig
    );

    // Wait for indexer to process
    sleep(Duration::from_secs(3)).await;

    // ============ Scenario 5: Fully Compressed Mint (CreateMint + CompressAndCloseMint) ============
    // This creates a compressed mint and then compresses it, so full mint data is in the compressed DB.
    // This is for testing getMintInterface cold path (no on-chain data needed).
    println!("\n=== Creating fully compressed mint ===");

    let compressed_mint_seed = Keypair::new();

    // Derive compression address for fully compressed mint
    let compressed_mint_compression_address =
        derive_mint_compressed_address(&compressed_mint_seed.pubkey(), &mint_address_tree.tree);

    let (compressed_mint_pda, compressed_mint_bump) =
        find_mint_address(&compressed_mint_seed.pubkey());

    // Get validity proof for the new address
    let rpc_result = rpc
        .get_validity_proof(
            vec![],
            vec![AddressWithTree {
                address: compressed_mint_compression_address,
                tree: mint_address_tree.tree,
            }],
            None,
        )
        .await
        .unwrap()
        .value;

    // Create compressed mint (will be decompressed initially)
    let compressed_mint_params = CreateMintParams {
        decimals: 9,
        address_merkle_tree_root_index: rpc_result.addresses[0].root_index,
        mint_authority: payer.pubkey(),
        proof: rpc_result.proof.0.unwrap(),
        compression_address: compressed_mint_compression_address,
        mint: compressed_mint_pda,
        bump: compressed_mint_bump,
        freeze_authority: Some(payer.pubkey()), // Add freeze authority for variety
        extensions: None,
        rent_payment: 0, // Immediately compressible
        write_top_up: 0,
    };

    let create_compressed_mint_builder = CreateMint::new(
        compressed_mint_params,
        compressed_mint_seed.pubkey(),
        payer.pubkey(),
        mint_address_tree.tree,
        output_queue,
    );
    let ix = create_compressed_mint_builder.instruction().unwrap();

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer, &compressed_mint_seed],
        blockhash,
    );
    let create_mint_sig = rpc.process_transaction(tx).await.unwrap();
    println!(
        "Created mint (step 1/2): {} (sig: {})",
        compressed_mint_pda, create_mint_sig
    );

    // Wait for indexer to process the CreateMint
    sleep(Duration::from_secs(3)).await;
    wait_for_indexer(&rpc).await.unwrap();

    // Now compress and close the mint to make it fully compressed
    println!("Compressing mint via CompressAndCloseMint...");

    let compress_params = MintActionParams {
        compressed_mint_address: compressed_mint_compression_address,
        mint_seed: compressed_mint_seed.pubkey(),
        authority: payer.pubkey(),
        payer: payer.pubkey(),
        actions: vec![MintActionType::CompressAndCloseMint { idempotent: false }],
        new_mint: None,
    };

    let compress_ix = create_mint_action_instruction(&mut rpc, compress_params)
        .await
        .expect("Failed to create CompressAndCloseMint instruction");

    let (blockhash, _) = rpc.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[compress_ix],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );
    let compress_mint_sig = rpc.process_transaction(tx).await.unwrap();
    println!(
        "Compressed mint (step 2/2): {} (sig: {})",
        compressed_mint_pda, compress_mint_sig
    );

    // Wait for indexer to process
    sleep(Duration::from_secs(3)).await;

    // ============ Scenario 6: Compressible Token Account ============
    println!("\n=== Creating compressible token account ===");

    let compressible_owner = Keypair::new();
    rpc.airdrop_lamports(&compressible_owner.pubkey(), 1_000_000_000)
        .await
        .expect("Failed to airdrop to compressible owner");

    let compressible_token_account = create_compressible_token_account(
        &mut rpc,
        CreateCompressibleTokenAccountInputs {
            owner: compressible_owner.pubkey(),
            mint: mint_pubkey,
            num_prepaid_epochs: 2,
            payer: &payer,
            token_account_keypair: None,
            lamports_per_write: Some(100),
            token_account_version: TokenDataVersion::ShaFlat,
        },
    )
    .await
    .expect("Failed to create compressible token account");
    println!(
        "Created compressible token account: {}",
        compressible_token_account
    );
    println!("Compressible owner: {}", compressible_owner.pubkey());

    // ============ Summary ============
    println!("\n========== ADDRESSES SUMMARY ==========\n");
    println!("SPL Mint: {}", mint_pubkey);
    println!("Registered v2 Address: {}", hex::encode(derived_address));
    println!(
        "Decompressed Mint PDA (on-chain CMint): {}",
        decompressed_mint_pda
    );
    println!(
        "Decompressed Mint Address: {:?}",
        decompressed_mint_compression_address
    );
    println!(
        "Fully Compressed Mint PDA (in compressed DB): {}",
        compressed_mint_pda
    );
    println!(
        "Fully Compressed Mint Address: {:?}",
        compressed_mint_compression_address
    );
    println!("Bob (compressed token holder): {}", bob.pubkey());
    println!("Charlie (compressed token holder): {}", charlie.pubkey());
    println!("Compressible owner: {}", compressible_owner.pubkey());
    println!("Compressible token account: {}", compressible_token_account);

    // ============ Test Interface Endpoints ============
    println!("\n========== TESTING INTERFACE ENDPOINTS ==========\n");

    // Create PhotonIndexer to test the interface endpoints
    let photon_indexer = PhotonIndexer::new("http://localhost:8784".to_string(), None);

    // Wait for indexer to sync
    sleep(Duration::from_secs(3)).await;
    wait_for_indexer(&rpc).await.unwrap();

    // ============ Test 1: getMintInterface with decompressed mint (on-chain CMint) ============
    println!("Test 1: getMintInterface with decompressed mint (on-chain CMint)...");
    let decompressed_mint_interface = photon_indexer
        .get_mint_interface(&decompressed_mint_pda, None)
        .await
        .expect("getMintInterface should not error for decompressed mint")
        .value
        .expect("Decompressed mint should be found");

    assert!(
        decompressed_mint_interface.account.is_hot(),
        "Decompressed mint should be hot (on-chain)"
    );
    assert!(
        decompressed_mint_interface.account.cold.is_none(),
        "On-chain mint should not have cold context"
    );
    assert_eq!(
        decompressed_mint_interface.account.key, decompressed_mint_pda,
        "Key should match the queried address"
    );
    assert!(
        decompressed_mint_interface.account.account.lamports > 0,
        "On-chain mint should have lamports > 0"
    );
    assert_eq!(
        decompressed_mint_interface.mint_data.decimals, 6,
        "Decompressed mint decimals should be 6"
    );
    assert_eq!(
        decompressed_mint_interface.mint_data.mint_pda, decompressed_mint_pda,
        "Mint PDA should match the queried address"
    );
    println!("  PASSED: Decompressed mint resolved from on-chain with correct data");

    // ============ Test 2: getMintInterface with fully compressed mint (compressed DB) ============
    println!("\nTest 2: getMintInterface with fully compressed mint (compressed DB)...");
    let compressed_mint_interface = photon_indexer
        .get_mint_interface(&compressed_mint_pda, None)
        .await
        .expect("getMintInterface should not error for compressed mint")
        .value
        .expect("Compressed mint should be found");

    assert!(
        compressed_mint_interface.account.is_cold(),
        "Fully compressed mint should be cold (from compressed DB)"
    );
    assert!(
        compressed_mint_interface.account.cold.is_some(),
        "Compressed mint should have cold context"
    );
    // Verify cold context is the Mint variant
    assert!(
        matches!(
            compressed_mint_interface.account.cold,
            Some(ColdContext::Mint { .. })
        ),
        "Cold context should be the Mint variant"
    );
    assert_eq!(
        compressed_mint_interface.account.key, compressed_mint_pda,
        "Key should match the queried address"
    );
    assert_eq!(
        compressed_mint_interface.mint_data.decimals, 9,
        "Compressed mint decimals should be 9"
    );
    assert_eq!(
        compressed_mint_interface.mint_data.freeze_authority,
        Some(payer.pubkey()),
        "Compressed mint freeze authority should match"
    );
    assert_eq!(
        compressed_mint_interface.mint_data.mint_pda, compressed_mint_pda,
        "Mint PDA should match the queried address"
    );
    println!("  PASSED: Compressed mint resolved from DB with correct data");

    // ============ Test 3: getAccountInterface with compressible token account (on-chain) ============
    println!("\nTest 3: getAccountInterface with compressible token account (on-chain)...");
    let compressible_account_interface = photon_indexer
        .get_account_interface(&compressible_token_account, None)
        .await
        .expect("getAccountInterface should not error for compressible account")
        .value
        .expect("Compressible token account should be found");

    assert!(
        compressible_account_interface.is_hot(),
        "Compressible account should be hot (on-chain)"
    );
    assert!(
        compressible_account_interface.cold.is_none(),
        "On-chain account should not have cold context"
    );
    assert_eq!(
        compressible_account_interface.key, compressible_token_account,
        "Key should match the queried address"
    );
    assert!(
        compressible_account_interface.account.lamports > 0,
        "On-chain account should have lamports > 0"
    );
    println!("  PASSED: Compressible account resolved from on-chain");

    // ============ Test 4: getTokenAccountInterface with compressible token account (on-chain) ============
    println!("\nTest 4: getTokenAccountInterface with compressible token account (on-chain)...");
    let compressible_token_interface = photon_indexer
        .get_token_account_interface(&compressible_token_account, None)
        .await
        .expect("getTokenAccountInterface should not error")
        .value
        .expect("Compressible token account should be found via token interface");

    assert!(
        compressible_token_interface.account.is_hot(),
        "Token account should be hot (on-chain)"
    );
    assert!(
        compressible_token_interface.account.cold.is_none(),
        "On-chain token account should not have cold context"
    );
    assert_eq!(
        compressible_token_interface.account.key, compressible_token_account,
        "Token account key should match"
    );
    assert_eq!(
        compressible_token_interface.token.mint, mint_pubkey,
        "Token mint should match SPL mint"
    );
    assert_eq!(
        compressible_token_interface.token.owner,
        compressible_owner.pubkey(),
        "Token owner should match compressible owner"
    );
    println!("  PASSED: Token account interface resolved with correct token data");

    // ============ Test 5: getMultipleAccountInterfaces batch lookup ============
    println!("\nTest 5: getMultipleAccountInterfaces batch lookup...");
    let batch_addresses = vec![&decompressed_mint_pda, &compressible_token_account];

    let batch_response = photon_indexer
        .get_multiple_account_interfaces(batch_addresses.clone(), None)
        .await
        .expect("getMultipleAccountInterfaces should not error");

    assert_eq!(
        batch_response.value.len(),
        2,
        "Batch response should have exactly 2 results"
    );

    // First result: decompressed mint
    let batch_mint = batch_response.value[0]
        .as_ref()
        .expect("Decompressed mint should be found in batch");
    assert!(batch_mint.is_hot(), "Batch mint should be hot (on-chain)");
    assert_eq!(
        batch_mint.key, decompressed_mint_pda,
        "Batch mint key should match"
    );
    assert!(
        batch_mint.account.lamports > 0,
        "Batch mint should have lamports > 0"
    );

    // Second result: compressible token account
    let batch_token = batch_response.value[1]
        .as_ref()
        .expect("Compressible account should be found in batch");
    assert!(
        batch_token.is_hot(),
        "Batch token account should be hot (on-chain)"
    );
    assert_eq!(
        batch_token.key, compressible_token_account,
        "Batch token account key should match"
    );
    assert!(
        batch_token.account.lamports > 0,
        "Batch token account should have lamports > 0"
    );
    println!("  PASSED: Batch lookup returned correct results");

    // ============ Test 6: Consistency between getMintInterface and getAccountInterface ============
    println!("\nTest 6: Consistency between getMintInterface and getAccountInterface...");
    let mint_via_mint = photon_indexer
        .get_mint_interface(&decompressed_mint_pda, None)
        .await
        .expect("getMintInterface should succeed")
        .value
        .expect("Mint should be found via getMintInterface");

    let mint_via_account = photon_indexer
        .get_account_interface(&decompressed_mint_pda, None)
        .await
        .expect("getAccountInterface should succeed")
        .value
        .expect("Mint should be found via getAccountInterface");

    assert_eq!(
        mint_via_mint.account.key, mint_via_account.key,
        "Keys should match between interfaces"
    );
    assert_eq!(
        mint_via_mint.account.account.lamports, mint_via_account.account.lamports,
        "Lamports should match between interfaces"
    );
    assert_eq!(
        mint_via_mint.account.cold.is_none(),
        mint_via_account.cold.is_none(),
        "Hot/cold status should match between interfaces"
    );
    assert_eq!(
        mint_via_mint.account.account.data, mint_via_account.account.data,
        "Data should match between interfaces"
    );
    assert_eq!(
        mint_via_mint.account.account.owner, mint_via_account.account.owner,
        "Owner should match between interfaces"
    );
    println!("  PASSED: Consistency verified between getMintInterface and getAccountInterface");

    // ============ Test 7: Verify fully compressed mint via getAccountInterface returns None ============
    // Fully compressed mints (after CompressAndCloseMint) have full mint data in the compressed DB.
    // Their address column contains the compression_address, not the mint_pda.
    // Since they don't have the [255; 8] discriminator, onchain_pubkey is not set.
    // Therefore getAccountInterface by mint_pda should return None (use getMintInterface instead).
    println!("\nTest 7: getAccountInterface with fully compressed mint PDA...");
    let compressed_via_account = photon_indexer
        .get_account_interface(&compressed_mint_pda, None)
        .await
        .expect("getAccountInterface should not error");

    assert!(
        compressed_via_account.value.is_none(),
        "Fully compressed mint should NOT be found via getAccountInterface (use getMintInterface)"
    );
    println!("  PASSED: Fully compressed mint correctly returns None via getAccountInterface");

    // ============ Test 8: Verify decompressed mint found via getAccountInterface (generic linking) ============
    // Decompressed mints have discriminator [255; 8] + 32-byte mint_pda in data.
    // The generic linking feature extracts this as onchain_pubkey during ingestion.
    // Therefore getAccountInterface(mint_pda) should find it via onchain_pubkey column.
    println!("\nTest 8: getAccountInterface with decompressed mint PDA (generic linking)...");
    let decompressed_via_account = photon_indexer
        .get_account_interface(&decompressed_mint_pda, None)
        .await
        .expect("getAccountInterface should not error");

    let decompressed_account = decompressed_via_account
        .value
        .expect("Decompressed mint should be found via getAccountInterface (generic linking)");

    // The decompressed mint should be found from on-chain (CMint account exists)
    assert!(
        decompressed_account.is_hot(),
        "Decompressed mint via getAccountInterface should be hot (on-chain)"
    );
    assert!(
        decompressed_account.cold.is_none(),
        "Decompressed mint via getAccountInterface should not have cold context"
    );
    assert_eq!(
        decompressed_account.key, decompressed_mint_pda,
        "Key should match the queried mint PDA"
    );
    assert!(
        decompressed_account.account.lamports > 0,
        "Decompressed mint should have lamports > 0"
    );
    println!("  PASSED: Decompressed mint found via getAccountInterface with generic linking");

    println!("\n========== ALL TESTS PASSED ==========");
    println!("\nTo export transactions, run:");
    println!("cargo xtask export-photon-test-data --test-name indexer_interface");
}
