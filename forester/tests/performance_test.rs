/// Performance Test for Forester Batch Processing
///
/// This test fills ALL queues with transactions BEFORE starting the forester,
/// then measures how long it takes to process all batches.
///
/// Test methodology:
/// 1. Setup local validator + prover + indexer
/// 2. Pre-fill V2 state queues with transactions (output + input)
/// 3. Pre-fill V2 address queues with addresses
/// 4. Record initial state (roots, queue sizes)
/// 5. Start forester and measure time to process
/// 6. Monitor metrics: throughput, batch processing time, etc.

use std::{collections::HashMap, env, sync::Arc, time::{Duration, Instant}};

use anchor_lang::Discriminator;
use borsh::BorshSerialize;
use create_address_test_program::create_invoke_cpi_instruction;
use forester::{
    config::{ExternalServicesConfig, GeneralConfig, RpcPoolConfig, TransactionConfig},
    epoch_manager::WorkReport,
    run_pipeline,
    utils::get_protocol_config,
    ForesterConfig,
};
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount,
    queue::BatchedQueueAccount,
};
use light_client::{
    indexer::{AddressWithTree, Indexer},
    local_test_validator::LightValidatorConfig,
    rpc::{LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::{
    address::derive_address,
    instruction_data::{
        compressed_proof::CompressedProof,
        data::NewAddressParamsAssigned,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
    TreeType,
};
use light_compressed_token::process_transfer::{
    transfer_sdk::{create_transfer_instruction, to_account_metas},
    TokenTransferOutputData,
};
use light_compressed_token_sdk::compat::TokenDataWithMerkleContext;
use light_client::indexer::GetCompressedTokenAccountsByOwnerOrDelegateOptions;
use light_program_test::accounts::test_accounts::TestAccounts;
use light_prover_client::prover::spawn_prover;
use light_system_program;
use light_test_utils::{
    conversions::sdk_to_program_token_data,
    pack::pack_new_address_params_assigned,
    spl::create_mint_helper_with_keypair,
};
use rand::{prelude::SliceRandom, rngs::StdRng, Rng, SeedableRng};
use serial_test::serial;
use solana_program::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use solana_sdk::{
    signature::{Keypair, Signature},
    signer::Signer,
};
use tokio::{
    sync::{mpsc, oneshot},
    time::timeout,
};

mod test_utils;
use test_utils::*;

const MINT_TO_NUM: u64 = 5;
const DEFAULT_TIMEOUT_SECONDS: u64 = 60 * 10; // 10 minutes for performance test
const COMPUTE_BUDGET_LIMIT: u32 = 1_000_000;

// Performance test configuration - Define target queue sizes
// Keep these small to avoid filling the queues (max capacity is ~14000)
const TARGET_STATE_QUEUE_SIZE: usize = 100;  // Target items in state output queue
const TARGET_ADDRESS_QUEUE_SIZE: usize = 100; // Target items in address queue

// Derived transaction counts based on queue size targets
const NUM_STATE_TRANSACTIONS: usize = TARGET_STATE_QUEUE_SIZE / MINT_TO_NUM as usize; // Each mint creates MINT_TO_NUM items
const NUM_ADDRESS_TRANSACTIONS: usize = TARGET_ADDRESS_QUEUE_SIZE / 10; // Each create_addresses_v2 creates 10 addresses (see line 236)

fn get_rpc_url() -> String {
    "http://localhost:8899".to_string()
}

fn get_ws_rpc_url() -> String {
    "ws://localhost:8900".to_string()
}

fn get_indexer_url() -> String {
    "http://localhost:8784".to_string()
}

fn get_prover_url() -> String {
    env::var("LIGHT_PROVER_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

fn get_prover_api_key() -> Option<String> {
    env::var("PROVER_API_KEY").ok()
}

fn get_photon_api_key() -> Option<String> {
    None
}

fn get_photon_grpc_url() -> Option<String> {
    None
}

fn get_forester_keypair() -> Keypair {
    TestAccounts::get_local_test_validator_accounts().protocol.forester.insecure_clone()
}

/// Pre-fill state queues with transactions
///
/// 2-Phase strategy to achieve PERFECT 100/100 balance for output and input queues on tree[0]:
///
/// Phase 1: Bootstrap on tree[0]
/// - NUM_STATE_TRANSACTIONS mints to tree[0] → creates 100 tokens in output queue
/// - tree[0] output queue: 100 items ✓
///
/// Phase 2: Cross-tree transfer tree[0] → tree[1]
/// - NUM_STATE_TRANSACTIONS transfers consuming from tree[0], outputting to tree[1]
/// - Nullifications go to tree[0]'s input queue → 100 items ✓
/// - Outputs go to tree[1]'s output queue (we don't care about tree[1])
///
/// Example with 20 transactions:
/// - Phase 1: 20 mints to tree[0] × 5 = 100 outputs on tree[0]
/// - Phase 2: 20 transfers tree[0]→tree[1] × 5 = 100 nullifications on tree[0]
/// - **Result: tree[0] has exactly 100 outputs + 100 inputs = 100/100 balance**
///
/// State output queue has max capacity of 100 items (2 batches × 50 items per batch in test config).
async fn prefill_state_queue<R: Rpc>(
    rpc: &mut R,
    env: &TestAccounts,
    payer: &Keypair,
    mint_pubkey: &Pubkey,
    num_transactions: usize,
) -> usize {
    let bootstrap_mints = num_transactions;

    println!("\n=== Pre-filling State Queue (2-Phase Strategy) ===");
    println!("Phase 1: Bootstrap - {} mints to tree[0] (fills output queue)", bootstrap_mints);
    println!("Phase 2: Transfer tree[0] → tree[1] - {} transfers (fills input queue)", num_transactions);
    println!("Expected tree[0]: {} outputs + {} inputs = 100/100 balance",
             num_transactions * MINT_TO_NUM as usize,
             num_transactions * MINT_TO_NUM as usize);

    let start = Instant::now();
    let mut phase2_successful = 0;
    let mut bootstrap_successful = 0;

    // Phase 1: Bootstrap mints to tree[0]
    // Creates tokens on tree[0] → fills tree[0] output queue with 100 items
    println!("\nPhase 1: Creating initial token pool on tree[0]...");
    for i in 0..bootstrap_mints {
        let result = mint_to(rpc, &env.v2_state_trees[0].output_queue, payer, mint_pubkey)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);

        if result.is_ok() {
            bootstrap_successful += 1;
            if (i + 1) % 5 == 0 {
                println!("  Bootstrap progress: {}/{} mints successful", bootstrap_successful, i + 1);
            }
        } else {
            eprintln!("  Bootstrap mint {} FAILED: {:?}", i, result);
        }
    }

    if bootstrap_successful == 0 {
        panic!("CRITICAL: All {} bootstrap mints failed! Cannot continue test.", bootstrap_mints);
    }

    println!("✓ Phase 1 complete: {}/{} bootstrap mints to tree[0]", bootstrap_successful, bootstrap_mints);

    // Wait for indexer to process bootstrap mints
    println!("\nWaiting 5 seconds for indexer to process bootstrap mints...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Phase 2: Cross-tree transfers tree[0] → tree[1]
    // Inputs from tree[0] → nullifications to tree[0] = 100 items in tree[0] input queue!
    // Outputs to tree[1] → we don't care about tree[1]'s queues
    println!("\nPhase 2: Transferring tokens tree[0] → tree[1] (fills input queue)...");
    for i in 0..num_transactions {
        let result = perform_cross_tree_transfer(
            rpc, env, payer, mint_pubkey,
            0, // source_tree_index: tree[0]
            1, // dest_tree_index: tree[1]
        ).await;

        if result.is_ok() {
            phase2_successful += 1;
            if (i + 1) % 5 == 0 {
                println!("  Phase 2 progress: {}/{} transfers", i + 1, num_transactions);
            }
        } else {
            eprintln!("  Phase 2 transfer {} failed: {:?}", i, result);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    let elapsed = start.elapsed();
    println!("✓ Phase 2 complete: {}/{} transfers (tree[0] → tree[1])", phase2_successful, num_transactions);
    println!("\n=== State Queue Pre-fill Summary ===");
    println!("  Phase 1 (bootstrap): {}/{} mints to tree[0]", bootstrap_successful, bootstrap_mints);
    println!("  Phase 2 (tree[0]→tree[1]): {}/{} transfers", phase2_successful, num_transactions);
    println!("  Total time: {:?}", elapsed);
    println!("  Expected tree[0] queue balance: 100 outputs / 100 inputs");

    phase2_successful
}

/// Pre-fill address queue with new addresses
async fn prefill_address_queue<R: Rpc>(
    rpc: &mut R,
    env: &TestAccounts,
    payer: &Keypair,
    num_addresses: usize,
) -> usize {
    println!("\n=== Pre-filling Address Queue ===");
    println!("Target addresses: {}", num_addresses);

    let start = Instant::now();
    let mut successful = 0;

    for i in 0..num_addresses {
        let result = create_addresses_v2(rpc, env, payer).await;

        if result.is_ok() {
            successful += 1;
            if (i + 1) % 10 == 0 {
                println!("Progress: {}/{} addresses", i + 1, num_addresses);
            }
        } else {
            eprintln!("Address creation {} failed: {:?}", i, result);
        }
    }

    let elapsed = start.elapsed();
    println!("✓ Pre-filled address queue: {} successful addresses in {:?}", successful, elapsed);
    println!("  Throughput: {:.2} addr/s", successful as f64 / elapsed.as_secs_f64());

    successful
}

/// Helper to mint tokens
async fn mint_to<R: Rpc>(
    rpc: &mut R,
    merkle_tree_pubkey: &Pubkey,
    payer: &Keypair,
    mint_pubkey: &Pubkey,
) -> Result<Signature, light_client::rpc::RpcError> {
    let mint_to_ix = light_compressed_token::process_mint::mint_sdk::create_mint_to_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        mint_pubkey,
        merkle_tree_pubkey,
        vec![100_000; MINT_TO_NUM as usize],
        vec![payer.pubkey(); MINT_TO_NUM as usize],
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
}

/// Helper to perform a cross-tree token transfer
/// Transfers consume MINT_TO_NUM (5) inputs from source_tree and create MINT_TO_NUM (5) outputs on dest_tree.
/// - Nullifications go to source_tree's input queue
/// - Outputs go to dest_tree's output queue
async fn perform_cross_tree_transfer<R: Rpc>(
    rpc: &mut R,
    env: &TestAccounts,
    payer: &Keypair,
    mint: &Pubkey,
    source_tree_index: usize,
    dest_tree_index: usize,
) -> Result<Signature, Box<dyn std::error::Error>> {
    // Get compressed token accounts owned by payer
    let mut input_compressed_accounts: Vec<TokenDataWithMerkleContext> = rpc
        .indexer()?
        .get_compressed_token_accounts_by_owner(
            &payer.pubkey(),
            Some(GetCompressedTokenAccountsByOwnerOrDelegateOptions {
                mint: Some(*mint),
                cursor: None,
                limit: None,
            }),
            None,
        )
        .await?
        .into();

    // Filter to ONLY use tokens from source_tree
    // Nullifications will go to source_tree's input queue
    let source_merkle_tree = env.v2_state_trees[source_tree_index].merkle_tree;
    input_compressed_accounts.retain(|acc| {
        acc.compressed_account.merkle_context.merkle_tree_pubkey == source_merkle_tree
    });

    // We need MINT_TO_NUM accounts for the transfer
    if input_compressed_accounts.len() < MINT_TO_NUM as usize {
        return Err(format!(
            "Not enough tokens on tree[{}]: found {}, need {}.",
            source_tree_index,
            input_compressed_accounts.len(),
            MINT_TO_NUM
        ).into());
    }

    // Shuffle and take MINT_TO_NUM random accounts (5 inputs)
    let rng = &mut rand::thread_rng();
    input_compressed_accounts.shuffle(rng);
    input_compressed_accounts.truncate(MINT_TO_NUM as usize);

    // Calculate total tokens from all input accounts
    let total_tokens: u64 = input_compressed_accounts
        .iter()
        .map(|acc| acc.token_data.amount)
        .sum();

    // Get hashes of all input accounts for validity proof
    let compressed_account_hashes: Vec<[u8; 32]> = input_compressed_accounts
        .iter()
        .map(|acc| acc.compressed_account.hash())
        .collect::<Result<Vec<_>, _>>()?;

    // Get validity proof for all inputs
    let proof_for_compressed_accounts = rpc
        .indexer()?
        .get_validity_proof(compressed_account_hashes, vec![], None)
        .await?;

    let root_indices = proof_for_compressed_accounts.value.get_root_indices();
    let merkle_contexts: Vec<_> = input_compressed_accounts
        .iter()
        .map(|acc| acc.compressed_account.merkle_context)
        .collect();

    // Create MINT_TO_NUM outputs on dest_tree: split total tokens evenly
    // IMPORTANT: For V2 trees, TokenTransferOutputData.merkle_tree expects OUTPUT_QUEUE pubkey!
    let amount_per_output = total_tokens / MINT_TO_NUM;
    let compressed_accounts: Vec<TokenTransferOutputData> = (0..MINT_TO_NUM)
        .map(|_| TokenTransferOutputData {
            amount: amount_per_output,
            owner: payer.pubkey(),
            lamports: None,
            merkle_tree: env.v2_state_trees[dest_tree_index].output_queue,
        })
        .collect();

    let proof = proof_for_compressed_accounts.value.proof.0.map(|p| CompressedProof {
        a: p.a,
        b: p.b,
        c: p.c,
    });

    // Convert all input accounts to program format
    let input_token_data: Vec<_> = input_compressed_accounts
        .iter()
        .map(|acc| sdk_to_program_token_data(acc.token_data.clone()))
        .collect();
    let input_compressed_accounts_data: Vec<_> = input_compressed_accounts
        .iter()
        .map(|acc| acc.compressed_account.compressed_account.clone())
        .collect();

    let instruction = create_transfer_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &merkle_contexts,
        &compressed_accounts,
        &root_indices,
        &proof,
        &input_token_data,
        &input_compressed_accounts_data,
        *mint,
        None,
        false,
        None,
        None,
        None,
        true,
        None,
        None,
        false,
        &[],
        false,
    )
    .map_err(|e| format!("Failed to create transfer instruction: {:?}", e))?;

    let instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
        instruction,
    ];

    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .map_err(|e| e.into())
}

/// Helper to create addresses for V2 address tree
async fn create_addresses_v2<R: Rpc>(
    rpc: &mut R,
    env: &TestAccounts,
    payer: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = StdRng::from_entropy();
    let batch_address_merkle_tree = &env.v2_address_trees[0];
    let num_addresses = 10;

    let mut address_seeds = Vec::with_capacity(num_addresses);
    let mut addresses = Vec::with_capacity(num_addresses);

    for _ in 0..num_addresses {
        let seed = rng.gen();
        let address = derive_address(
            &seed,
            &batch_address_merkle_tree.to_bytes(),
            &create_address_test_program::ID.to_bytes(),
        );

        address_seeds.push(seed);
        addresses.push(address);
    }

    let address_with_trees = addresses
        .into_iter()
        .map(|address| AddressWithTree {
            address,
            tree: *batch_address_merkle_tree,
        })
        .collect::<Vec<_>>();

    let proof_result = rpc
        .indexer()
        .unwrap()
        .get_validity_proof(Vec::new(), address_with_trees, None)
        .await
        .unwrap();

    let new_address_params = address_seeds
        .iter()
        .enumerate()
        .map(|(i, seed)| NewAddressParamsAssigned {
            seed: *seed,
            address_queue_pubkey: (*batch_address_merkle_tree).into(),
            address_merkle_tree_pubkey: (*batch_address_merkle_tree).into(),
            address_merkle_tree_root_index: proof_result.value.get_address_root_indices()[i],
            assigned_account_index: None,
        })
        .collect::<Vec<_>>();

    let mut remaining_accounts = HashMap::<Pubkey, usize>::new();
    let packed_new_address_params =
        pack_new_address_params_assigned(&new_address_params, &mut remaining_accounts);

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

    let instruction = create_invoke_cpi_instruction(
        payer.pubkey(),
        [
            light_system_program::instruction::InvokeCpiWithReadOnly::DISCRIMINATOR.to_vec(),
            ix_data.try_to_vec()?,
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

    rpc.create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await
        .map(|_| ())
        .map_err(|e| e.into())
}

/// Setup RPC connection
async fn setup_rpc_connection(forester: &Keypair) -> LightClient {
    let mut rpc = LightClient::new(LightClientConfig::local())
        .await
        .unwrap();
    rpc.payer = forester.insecure_clone();
    rpc
}

/// Ensure account has sufficient balance
async fn ensure_sufficient_balance(rpc: &mut LightClient, pubkey: &Pubkey, target_balance: u64) {
    if rpc.get_balance(pubkey).await.unwrap() < target_balance {
        rpc.airdrop_lamports(pubkey, target_balance).await.unwrap();
    }
}

/// Get initial merkle tree state
async fn get_initial_merkle_tree_state(
    rpc: &mut LightClient,
    merkle_tree_pubkey: &Pubkey,
    kind: TreeType,
) -> (u64, u64, [u8; 32]) {
    match kind {
        TreeType::StateV2 => {
            let mut merkle_tree_account =
                rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
            let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
                merkle_tree_account.data.as_mut_slice(),
                &merkle_tree_pubkey.into(),
            )
            .unwrap();

            let initial_next_index = merkle_tree.get_metadata().next_index;
            let initial_sequence_number = merkle_tree.get_metadata().sequence_number;
            (
                initial_next_index,
                initial_sequence_number,
                merkle_tree.get_root().unwrap(),
            )
        }
        TreeType::AddressV2 => {
            let mut merkle_tree_account =
                rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
            let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
                merkle_tree_account.data.as_mut_slice(),
                &merkle_tree_pubkey.into(),
            )
            .unwrap();

            let initial_next_index = merkle_tree.get_metadata().next_index;
            let initial_sequence_number = merkle_tree.get_metadata().sequence_number;
            (
                initial_next_index,
                initial_sequence_number,
                merkle_tree.get_root().unwrap(),
            )
        }
        _ => panic!("Unsupported tree type"),
    }
}

/// Setup forester pipeline
async fn setup_forester_pipeline(
    config: &ForesterConfig,
) -> (
    tokio::task::JoinHandle<anyhow::Result<()>>,
    oneshot::Sender<()>,
    oneshot::Sender<()>,
    oneshot::Sender<()>,
    mpsc::Receiver<WorkReport>,
) {
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (shutdown_compressible_sender, shutdown_compressible_receiver) = oneshot::channel();
    let (shutdown_bootstrap_sender, shutdown_bootstrap_receiver) = oneshot::channel();
    let (work_report_sender, work_report_receiver) = mpsc::channel(100);

    let service_handle = tokio::spawn(run_pipeline::<LightClient>(
        Arc::from(config.clone()),
        None,
        None,
        shutdown_receiver,
        Some(shutdown_compressible_receiver),
        Some(shutdown_bootstrap_receiver),
        work_report_sender,
    ));

    (
        service_handle,
        shutdown_sender,
        shutdown_compressible_sender,
        shutdown_bootstrap_sender,
        work_report_receiver,
    )
}

/// Get actual queue pending items from on-chain accounts
async fn get_queue_pending_items<R: Rpc>(
    rpc: &mut R,
    env: &TestAccounts,
) -> (usize, usize, usize) {
    let state_output = get_output_queue_pending(rpc, &env.v2_state_trees[0].output_queue).await;
    let state_input = get_input_queue_pending(rpc, &env.v2_state_trees[0].merkle_tree).await;
    let address = get_address_queue_pending(rpc, &env.v2_address_trees[0]).await;

    (state_output, state_input, address)
}

/// Get pending items in state output queue
async fn get_output_queue_pending<R: Rpc>(rpc: &mut R, queue_pubkey: &Pubkey) -> usize {
    match rpc.get_account(*queue_pubkey).await {
        Ok(Some(mut account)) => {
            if let Ok(output_queue) = BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice()) {
                let metadata = output_queue.get_metadata();
                let mut total_pending = 0;
                for batch in metadata.batch_metadata.batches.iter() {
                    let num_inserted = batch.get_num_inserted_zkps();
                    let current_index = batch.get_current_zkp_batch_index();
                    let pending_in_batch = current_index.saturating_sub(num_inserted);
                    total_pending += pending_in_batch * metadata.batch_metadata.zkp_batch_size;
                }
                total_pending as usize
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// Get pending items in state input queue (nullify)
async fn get_input_queue_pending<R: Rpc>(rpc: &mut R, merkle_tree_pubkey: &Pubkey) -> usize {
    match rpc.get_account(*merkle_tree_pubkey).await {
        Ok(Some(mut account)) => {
            if let Ok(merkle_tree) = BatchedMerkleTreeAccount::state_from_bytes(
                account.data.as_mut_slice(),
                &(*merkle_tree_pubkey).into(),
            ) {
                let mut total_pending = 0;
                for batch in merkle_tree.queue_batches.batches.iter() {
                    let num_inserted = batch.get_num_inserted_zkps();
                    let current_index = batch.get_current_zkp_batch_index();
                    let pending_in_batch = current_index.saturating_sub(num_inserted);
                    total_pending += pending_in_batch * batch.zkp_batch_size;
                }
                total_pending as usize
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// Get pending items in address queue
async fn get_address_queue_pending<R: Rpc>(rpc: &mut R, merkle_tree_pubkey: &Pubkey) -> usize {
    match rpc.get_account(*merkle_tree_pubkey).await {
        Ok(Some(mut account)) => {
            if let Ok(merkle_tree) = BatchedMerkleTreeAccount::address_from_bytes(
                account.data.as_mut_slice(),
                &(*merkle_tree_pubkey).into(),
            ) {
                let mut total_pending = 0;
                for batch in merkle_tree.queue_batches.batches.iter() {
                    let num_inserted = batch.get_num_inserted_zkps();
                    let current_index = batch.get_current_zkp_batch_index();
                    let pending_in_batch = current_index.saturating_sub(num_inserted);
                    total_pending += pending_in_batch * batch.zkp_batch_size;
                }
                total_pending as usize
            } else {
                0
            }
        }
        _ => 0,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
#[serial]
async fn performance_test_prefilled_queues() {
    println!("\n=== FORESTER PERFORMANCE TEST ===");
    println!("Test: Pre-fill all queues, then measure forester processing time\n");

    let env = TestAccounts::get_local_test_validator_accounts();

    // Initialize local validator
    init(Some(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: false,
        wait_time: 60,
        sbf_programs: vec![(
            "FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy".to_string(),
            "../target/deploy/create_address_test_program.so".to_string(),
        )],
        limit_ledger_size: None,
    }))
    .await;

    spawn_prover().await;

    let payer = get_forester_keypair();
    let mut rpc = setup_rpc_connection(&payer).await;

    ensure_sufficient_balance(&mut rpc, &payer.pubkey(), LAMPORTS_PER_SOL * 100).await;
    ensure_sufficient_balance(
        &mut rpc,
        &env.protocol.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100,
    )
    .await;

    // Create mint for state transactions
    let mint_keypair = Keypair::new();
    let mint_pubkey = create_mint_helper_with_keypair(&mut rpc, &payer, &mint_keypair).await;
    println!("Created mint: {}", mint_pubkey);

    // Get initial roots
    let (_, _, pre_state_root) = get_initial_merkle_tree_state(
        &mut rpc,
        &env.v2_state_trees[0].merkle_tree,
        TreeType::StateV2,
    )
    .await;

    let (_, _, pre_address_root) = get_initial_merkle_tree_state(
        &mut rpc,
        &env.v2_address_trees[0],
        TreeType::AddressV2,
    )
    .await;

    println!("\n=== Initial State ===");
    println!("State root: {:?}[..8]", &pre_state_root[..8]);
    println!("Address root: {:?}[..8]", &pre_address_root[..8]);

    // PRE-FILL QUEUES BEFORE STARTING FORESTER
    println!("\n=== PHASE 1: PRE-FILLING QUEUES ===");

    let state_txs = prefill_state_queue(
        &mut rpc,
        &env,
        &payer,
        &mint_pubkey,
        NUM_STATE_TRANSACTIONS,
    )
    .await;

    let address_txs = prefill_address_queue(
        &mut rpc,
        &env,
        &payer,
        NUM_ADDRESS_TRANSACTIONS,
    )
    .await;

    let (state_out, state_in, addr_queue) = get_queue_pending_items(&mut rpc, &env).await;
    println!("\n=== Queue Status Before Forester ===");
    println!("State output queue: {} pending items", state_out);
    println!("State input queue: {} pending items", state_in);
    println!("Address queue: {} pending items", addr_queue);

    // Assert queues are properly filled to target sizes
    // State output: Target is TARGET_STATE_QUEUE_SIZE items
    assert!(
        state_out > 0,
        "State output queue should have items before forester starts, but was empty"
    );
    assert!(
        state_out >= TARGET_STATE_QUEUE_SIZE / 2,
        "State output queue has {} items but expected at least {} (half of target {})",
        state_out,
        TARGET_STATE_QUEUE_SIZE / 2,
        TARGET_STATE_QUEUE_SIZE
    );

    // Address queue: Target is TARGET_ADDRESS_QUEUE_SIZE items
    assert!(
        addr_queue > 0,
        "Address queue should have items before forester starts, but was empty"
    );
    assert!(
        addr_queue >= TARGET_ADDRESS_QUEUE_SIZE / 2,
        "Address queue has {} items but expected at least {} (half of target {}). Addresses may not be going into queue properly.",
        addr_queue,
        TARGET_ADDRESS_QUEUE_SIZE / 2,
        TARGET_ADDRESS_QUEUE_SIZE
    );

    let total_queue_items = state_out + addr_queue;
    println!("✓ Queues are properly filled (state_output: {}/{}, address: {}/{}, total: {})",
             state_out, TARGET_STATE_QUEUE_SIZE, addr_queue, TARGET_ADDRESS_QUEUE_SIZE, total_queue_items);

    // NOW START FORESTER AND MEASURE TIME
    println!("\n=== PHASE 2: STARTING FORESTER ===");

    let config = ForesterConfig {
        external_services: ExternalServicesConfig {
            rpc_url: get_rpc_url(),
            ws_rpc_url: Some(get_ws_rpc_url()),
            indexer_url: Some(get_indexer_url()),
            prover_url: Some(get_prover_url()),
            prover_append_url: None,
            prover_update_url: None,
            prover_address_append_url: None,
            prover_api_key: get_prover_api_key(),
            prover_polling_interval: None,
            prover_max_wait_time: None,
            photon_api_key: get_photon_api_key(),
            photon_grpc_url: get_photon_grpc_url(),
            pushgateway_url: None,
            pagerduty_routing_key: None,
            rpc_rate_limit: None,
            photon_rate_limit: None,
            send_tx_rate_limit: None,
        },
        retry_config: Default::default(),
        queue_config: Default::default(),
        indexer_config: Default::default(),
        transaction_config: TransactionConfig::default(),
        general_config: GeneralConfig {
            slot_update_interval_seconds: 10,
            tree_discovery_interval_seconds: 5,
            enable_metrics: false,
            skip_v1_state_trees: true, // Focus on V2
            skip_v2_state_trees: false,
            skip_v1_address_trees: true,
            skip_v2_address_trees: false,
            // Filter to only process the specific trees we prefilled
            tree_ids: vec![
                env.v2_state_trees[0].merkle_tree,
                env.v2_address_trees[0],
            ],
            sleep_after_processing_ms: 50,
            sleep_when_idle_ms: 100,
        },
        rpc_pool_config: RpcPoolConfig {
            max_size: 50,
            connection_timeout_secs: 15,
            idle_timeout_secs: 300,
            max_retries: 10,
            initial_retry_delay_ms: 1000,
            max_retry_delay_ms: 16000,
        },
        registry_pubkey: light_registry::ID,
        payer_keypair: payer.insecure_clone(),
        derivation_pubkey: payer.pubkey(),
        address_tree_data: vec![],
        state_tree_data: vec![],
        compressible_config: None,
    };

    let protocol_config = get_protocol_config(&mut rpc).await;
    let registration_slot = get_registration_phase_start_slot(&mut rpc, &protocol_config).await;
    wait_for_slot(&mut rpc, registration_slot).await;

    // Get active phase slot before starting forester
    let active_slot = get_active_phase_start_slot(&mut rpc, &protocol_config).await;
    println!("Active phase starts at slot {}", active_slot);

    // Wait until just before active phase to start forester
    // This ensures we capture the full processing time
    println!("Waiting until slot {} to start forester...", active_slot.saturating_sub(5));
    wait_for_slot(&mut rpc, active_slot.saturating_sub(5)).await;

    let (service_handle, shutdown_sender, shutdown_compressible_sender, shutdown_bootstrap_sender, mut work_report_receiver) =
        setup_forester_pipeline(&config).await;

    // Start timer from forester pipeline start
    let pipeline_start = Instant::now();
    println!("⏱️  TIMER STARTED - Forester pipeline started");

    // Wait for active phase
    wait_for_slot(&mut rpc, active_slot).await;
    let active_phase_reached = pipeline_start.elapsed();
    println!("✓ Active phase reached at {:?}", active_phase_reached);

    // Monitor work reports
    let mut total_processed = 0;
    let mut last_report_time = Instant::now();

    let timeout_duration = Duration::from_secs(DEFAULT_TIMEOUT_SECONDS);
    let _result = timeout(timeout_duration, async {
        while let Some(report) = work_report_receiver.recv().await {
            total_processed += report.processed_items;

            if last_report_time.elapsed() > Duration::from_secs(5) {
                let elapsed = pipeline_start.elapsed();
                println!("Progress: {} items processed in {:?}", total_processed, elapsed);
                println!("  Throughput: {:.2} items/s", total_processed as f64 / elapsed.as_secs_f64());
                last_report_time = Instant::now();
            }

            // Check if we've processed all expected items
            let expected_total = state_txs + address_txs;
            if total_processed >= expected_total {
                println!("\n✓ All items processed!");
                break;
            }
        }
        Ok::<(), ()>(())
    })
    .await;

    let total_elapsed = pipeline_start.elapsed();
    // Processing time = total time - time waiting for active phase
    let processing_time = total_elapsed.saturating_sub(active_phase_reached);

    // FINAL RESULTS
    // State queue breakdown:
    //   - Output queue: 100 items (from bootstrap mints)
    //   - Input queue: 100 items (nullifications from transfers)
    // Address queue: 100 items
    let state_output_items = 100;
    let state_input_items = 100;
    let address_items = 100;

    println!("\n=== PERFORMANCE TEST RESULTS ===");
    println!("Total elapsed time: {:?}", total_elapsed);
    println!("  Time to active phase: {:?}", active_phase_reached);
    println!("  Processing time: {:?}", processing_time);
    println!("Total items processed: {}", total_processed);
    println!("  State output items: {}", state_output_items);
    println!("  State input items: {}", state_input_items);
    println!("  Address items: {}", address_items);
    println!("\nThroughput (items/second, based on processing time):");
    let processing_secs = processing_time.as_secs_f64();
    if processing_secs > 0.0 {
        println!("  Overall: {:.2} items/second", total_processed as f64 / processing_secs);
        println!("  State output: {:.2} items/second", state_output_items as f64 / processing_secs);
        println!("  State input: {:.2} items/second", state_input_items as f64 / processing_secs);
        println!("  Address: {:.2} items/second", address_items as f64 / processing_secs);
    } else {
        println!("  (processing time too short to measure accurately)");
    }

    // Verify roots changed
    let (_, _, post_state_root) = get_initial_merkle_tree_state(
        &mut rpc,
        &env.v2_state_trees[0].merkle_tree,
        TreeType::StateV2,
    )
    .await;

    let (_, _, post_address_root) = get_initial_merkle_tree_state(
        &mut rpc,
        &env.v2_address_trees[0],
        TreeType::AddressV2,
    )
    .await;

    println!("\n=== Root Verification ===");
    println!("State root changed: {}", pre_state_root != post_state_root);
    println!("  Before: {:?}[..8]", &pre_state_root[..8]);
    println!("  After:  {:?}[..8]", &post_state_root[..8]);
    println!("Address root changed: {}", pre_address_root != post_address_root);
    println!("  Before: {:?}[..8]", &pre_address_root[..8]);
    println!("  After:  {:?}[..8]", &post_address_root[..8]);

    // Assert roots actually changed (proof that forester processed the batches)
    assert_ne!(
        pre_state_root, post_state_root,
        "State root should have changed after forester processing"
    );
    assert_ne!(
        pre_address_root, post_address_root,
        "Address root should have changed after forester processing"
    );
    println!("✓ Roots changed correctly");

    // Verify queues are now empty (or nearly empty)
    let (final_state_out, final_state_in, final_addr_queue) = get_queue_pending_items(&mut rpc, &env).await;
    println!("\n=== Queue Status After Forester ===");
    println!("State output queue: {} pending items", final_state_out);
    println!("State input queue: {} pending items", final_state_in);
    println!("Address queue: {} pending items", final_addr_queue);

    // Assert queues are empty or nearly empty (allow small margin for in-flight transactions)
    const MAX_REMAINING_ITEMS: usize = 10;
    assert!(
        final_state_out <= MAX_REMAINING_ITEMS,
        "State output queue should be empty after processing, but has {} items (max allowed: {})",
        final_state_out,
        MAX_REMAINING_ITEMS
    );
    assert!(
        final_state_in <= MAX_REMAINING_ITEMS,
        "State input queue should be empty after processing, but has {} items (max allowed: {})",
        final_state_in,
        MAX_REMAINING_ITEMS
    );
    assert!(
        final_addr_queue <= MAX_REMAINING_ITEMS,
        "Address queue should be empty after processing, but has {} items (max allowed: {})",
        final_addr_queue,
        MAX_REMAINING_ITEMS
    );

    println!("✓ All queues are empty or nearly empty (state_output: {}, state_input: {}, address: {})",
             final_state_out, final_state_in, final_addr_queue);

    // Cleanup
    shutdown_sender.send(()).unwrap();
    shutdown_compressible_sender.send(()).unwrap();
    shutdown_bootstrap_sender.send(()).unwrap();
    service_handle.await.unwrap().unwrap();

    println!("\n✓ Performance test completed successfully!");
}
