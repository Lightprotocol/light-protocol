/// Test scenarios for indexer interface endpoints.
///
/// This test creates various account types for testing the indexer's interface racing logic.
/// After running, use `cargo xtask export-photon-test-data --test-name indexer_interface`
/// to export transactions to the indexer's test snapshot directory.
///
/// Scenarios covered:
/// 1. Light Token Mint - mint for token operations
/// 2. Token accounts (via light-token-client MintTo) - for getTokenAccountInterface
/// 3. Registered v2 address in batched address tree - for address tree verification
/// 4. Compressible token accounts - on-chain accounts that can be compressed
use std::collections::HashMap;

use anchor_lang::{AnchorDeserialize, Discriminator};
use borsh::BorshSerialize;
use create_address_test_program::create_invoke_cpi_instruction;
use light_client::{
    indexer::{photon_indexer::PhotonIndexer, AddressWithTree, Indexer},
    local_test_validator::{spawn_validator, LightValidatorConfig},
    rpc::{LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::{
    address::derive_address,
    instruction_data::{
        data::NewAddressParamsAssigned, with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
};
use light_compressed_token::process_transfer::transfer_sdk::to_account_metas;
use light_test_utils::{
    actions::legacy::{
        create_compressible_token_account,
        instructions::mint_action::{
            create_mint_action_instruction, MintActionParams, MintActionType,
        },
        CreateCompressibleTokenAccountInputs,
    },
    pack::pack_new_address_params_assigned,
};
use light_token::instruction::{
    derive_mint_compressed_address, find_mint_address, CreateMint as CreateMintInstruction,
    CreateMintParams,
};
use light_token_client::{CreateAta, CreateMint, MintTo};
use light_token_interface::state::TokenDataVersion;
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};
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
        wait_time: 0,
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

    println!("\n========== PHOTON INTERFACE TEST ==========\n");
    println!("Payer: {}", payer.pubkey());

    // ============ Scenario 1: Create Light Token Mint ============
    println!("\n=== Creating Light Token mint ===");

    let (create_mint_sig, mint_pubkey) = CreateMint {
        decimals: 9,
        ..Default::default()
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .expect("Failed to create Light Token mint");
    println!(
        "Light Token Mint: {} (sig: {})",
        mint_pubkey, create_mint_sig
    );

    // ============ Scenario 2: Mint tokens to Bob and Charlie ============
    println!("\n=== Minting tokens via light-token-client ===");

    let bob = Keypair::new();
    let charlie = Keypair::new();

    // Create ATAs for Bob and Charlie
    let (_, bob_ata) = CreateAta {
        mint: mint_pubkey,
        owner: bob.pubkey(),
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .expect("Failed to create Bob's ATA");

    let (_, charlie_ata) = CreateAta {
        mint: mint_pubkey,
        owner: charlie.pubkey(),
        idempotent: false,
    }
    .execute(&mut rpc, &payer)
    .await
    .expect("Failed to create Charlie's ATA");

    // Mint tokens
    let bob_mint_sig = MintTo {
        mint: mint_pubkey,
        destination: bob_ata,
        amount: 1_000_000_000,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .expect("Failed to mint to Bob");

    let charlie_mint_sig = MintTo {
        mint: mint_pubkey,
        destination: charlie_ata,
        amount: 500_000_000,
    }
    .execute(&mut rpc, &payer, &payer)
    .await
    .expect("Failed to mint to Charlie");

    println!("Minted to Bob: {} (sig: {})", bob.pubkey(), bob_mint_sig);
    println!(
        "Minted to Charlie: {} (sig: {})",
        charlie.pubkey(),
        charlie_mint_sig
    );

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
    let ix_data = InstructionDataInvokeCpiWithReadOnly::new(
        create_address_test_program::ID.into(),
        255,
        proof_result.value.proof.0,
    )
    .mode_v1()
    .with_with_transaction_hash(true)
    .with_new_addresses(&packed_new_address_params);

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
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
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

    // ============ Scenario 4: Decompressed Mint (CreateMint with rent_payment=2) ============
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
        rent_payment: 2, // Minimum required epochs of rent prepayment
        write_top_up: 0,
    };

    let create_decompressed_mint_builder = CreateMintInstruction::new(
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
        rent_payment: 2, // Minimum required epochs of rent prepayment
        write_top_up: 0,
    };

    let create_compressed_mint_builder = CreateMintInstruction::new(
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

    // Warp forward so rent expires - required before CompressAndCloseMint
    let current_slot = rpc.get_slot().await.unwrap();
    let target_slot = current_slot + light_compressible::rent::SLOTS_PER_EPOCH * 30;
    rpc.warp_to_slot(target_slot)
        .await
        .expect("warp_to_slot so mint rent expires");

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
            mint: decompressed_mint_pda,
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
    println!("Light Token Mint: {}", mint_pubkey);
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
    let photon_indexer = PhotonIndexer::new("http://localhost:8784".to_string());

    // ============ Test 1: getAccountInterface with compressible token account (on-chain) ============
    println!("Test 1: getAccountInterface with compressible token account (on-chain)...");
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

    println!("\nTest 2: getAccountInterface for compressible token account (on-chain)...");
    let compressible_token_interface = rpc
        .get_account_interface(&compressible_token_account, None)
        .await
        .expect("getAccountInterface should not error")
        .value
        .expect("Compressible token account should be found");

    assert!(
        compressible_token_interface.is_hot(),
        "Token account should be hot (on-chain)"
    );
    assert!(
        compressible_token_interface.cold.is_none(),
        "On-chain token account should not have cold context"
    );
    assert_eq!(
        compressible_token_interface.key, compressible_token_account,
        "Token account key should match"
    );
    {
        let token = light_token_interface::state::Token::deserialize(
            &mut &compressible_token_interface.account.data[..],
        )
        .expect("parse token account");
        assert_eq!(
            token.mint, decompressed_mint_pda,
            "Token mint should match decompressed mint"
        );
        assert_eq!(
            token.owner,
            compressible_owner.pubkey(),
            "Token owner should match compressible owner"
        );
    }
    println!("  PASSED: Token account interface resolved with correct token data");

    // ============ Test 3: getMultipleAccountInterfaces batch lookup ============
    println!("\nTest 3: getMultipleAccountInterfaces batch lookup...");
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

    // ============ Test 4: Verify fully compressed mint via getAccountInterface returns None ============
    // Fully compressed mints (after CompressAndCloseMint) have full mint data in the compressed DB.
    // Their address column contains the compression_address, not the mint_pda.
    // Since they don't have the [255; 8] discriminator, onchain_pubkey is not set.
    // Therefore getAccountInterface by mint_pda should return None.
    println!("\nTest 4: getAccountInterface with fully compressed mint PDA...");
    let compressed_via_account = photon_indexer
        .get_account_interface(&compressed_mint_pda, None)
        .await
        .expect("getAccountInterface should not error");

    assert!(
        compressed_via_account.value.is_none(),
        "Fully compressed mint should NOT be found via getAccountInterface"
    );
    println!("  PASSED: Fully compressed mint correctly returns None via getAccountInterface");

    // ============ Test 5: Verify decompressed mint found via getAccountInterface (generic linking) ============
    // Decompressed mints have discriminator [255; 8] + 32-byte mint_pda in data.
    // The generic linking feature extracts this as onchain_pubkey during ingestion.
    // Therefore getAccountInterface(mint_pda) should find it via onchain_pubkey column.
    println!("\nTest 5: getAccountInterface with decompressed mint PDA (generic linking)...");
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
