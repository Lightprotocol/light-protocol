use std::{
    collections::HashMap,
    env,
    sync::Arc,
    time::{Duration, Instant},
};

use anchor_lang::Discriminator;
use borsh::BorshSerialize;
use create_address_test_program::create_invoke_cpi_instruction;
use forester::{
    config::{ExternalServicesConfig, GeneralConfig, RpcPoolConfig, TransactionConfig},
    epoch_manager::{ProcessingMetrics, WorkReport},
    run_pipeline,
    utils::get_protocol_config,
    ForesterConfig,
};
use forester_utils::forester_epoch::get_epoch_phases;
use light_batched_merkle_tree::{
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_client::indexer::GetCompressedTokenAccountsByOwnerOrDelegateOptions;
use light_client::{
    indexer::{AddressWithTree, Indexer},
    local_test_validator::LightValidatorConfig,
    rpc::{LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::{
    address::derive_address,
    instruction_data::{
        compressed_proof::CompressedProof, data::NewAddressParamsAssigned,
        with_readonly::InstructionDataInvokeCpiWithReadOnly,
    },
    TreeType,
};
use light_compressed_token::process_transfer::{
    transfer_sdk::{create_transfer_instruction, to_account_metas},
    TokenTransferOutputData,
};
use light_compressed_token_sdk::compat::TokenDataWithMerkleContext;
use light_program_test::accounts::test_accounts::TestAccounts;
use light_prover_client::prover::spawn_prover;
use light_system_program;
use light_test_utils::{
    conversions::sdk_to_program_token_data, pack::pack_new_address_params_assigned,
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
use tracing_subscriber;

mod test_utils;
use test_utils::*;

const MINT_TO_NUM: u64 = 5;
const DEFAULT_TIMEOUT_SECONDS: u64 = 60 * 20;
const COMPUTE_BUDGET_LIMIT: u32 = 1_000_000;

const TARGET_STATE_QUEUE_SIZE: usize = 100;
const TARGET_ADDRESS_QUEUE_SIZE: usize = 100;

const NUM_STATE_TRANSACTIONS: usize = TARGET_STATE_QUEUE_SIZE / MINT_TO_NUM as usize;
const NUM_ADDRESS_TRANSACTIONS: usize = TARGET_ADDRESS_QUEUE_SIZE / 10;

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
    TestAccounts::get_local_test_validator_accounts()
        .protocol
        .forester
        .insecure_clone()
}

async fn prefill_state_queue<R: Rpc>(
    rpc: &mut R,
    env: &TestAccounts,
    payer: &Keypair,
    mint_pubkey: &Pubkey,
    num_transactions: usize,
) -> usize {
    let bootstrap_mints = num_transactions;

    println!("\n=== Pre-filling State Queue ===");
    println!(
        "Phase 1: Bootstrap - {} mints to tree[0] (fills output queue)",
        bootstrap_mints
    );
    println!(
        "Phase 2: Transfer tree[0] → tree[1] - {} transfers (fills input queue)",
        num_transactions
    );
    println!(
        "Expected tree[0]: {} outputs + {} inputs = {}/{} balance",
        TARGET_STATE_QUEUE_SIZE,
        TARGET_STATE_QUEUE_SIZE,
        num_transactions * MINT_TO_NUM as usize,
        num_transactions * MINT_TO_NUM as usize
    );

    let start = Instant::now();
    let mut phase2_successful = 0;
    let mut bootstrap_successful = 0;

    println!("\nPhase 1: Creating initial token pool on tree[0]...");
    for i in 0..bootstrap_mints {
        let result = mint_to(rpc, &env.v2_state_trees[0].output_queue, payer, mint_pubkey)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);

        if result.is_ok() {
            bootstrap_successful += 1;
            if (i + 1) % 5 == 0 {
                println!(
                    "  Bootstrap progress: {} mints successful",
                    bootstrap_successful
                );
            }
        } else {
            eprintln!("  Bootstrap mint {} FAILED: {:?}", i, result);
        }
    }

    if bootstrap_successful == 0 {
        panic!(
            "CRITICAL: All {} bootstrap mints failed! Cannot continue test.",
            bootstrap_mints
        );
    }

    println!(
        "Phase 1 complete: {}/{} bootstrap mints to tree[0]",
        bootstrap_successful, bootstrap_mints
    );

    println!("\nPhase 2: Transferring tokens tree[0] → tree[1] (fills input queue)...");
    for i in 0..num_transactions {
        let result = perform_cross_tree_transfer(
            rpc,
            env,
            payer,
            mint_pubkey,
            0, // source_tree_index: tree[0]
            1, // dest_tree_index: tree[1]
        )
        .await;

        if result.is_ok() {
            phase2_successful += 1;
            if (i + 1) % 5 == 0 {
                println!(
                    "  Phase 2 progress: {}/{} transfers",
                    i + 1,
                    num_transactions
                );
            }
        } else {
            eprintln!("  Phase 2 transfer {} failed: {:?}", i, result);
        }
    }

    let elapsed = start.elapsed();
    println!(
        "Phase 2 complete: {}/{} transfers (tree[0] → tree[1])",
        phase2_successful, num_transactions
    );
    println!("\n=== State Queue Pre-fill Summary ===");
    println!(
        "Phase 1 (bootstrap): {}/{} mints to tree[0]",
        bootstrap_successful, bootstrap_mints
    );
    println!(
        "Phase 2 (tree[0]→tree[1]): {}/{} transfers",
        phase2_successful, num_transactions
    );
    println!("  Total time: {:?}", elapsed);
    println!(
        "  Expected tree[0] queue balance: {} outputs / {} inputs",
        TARGET_STATE_QUEUE_SIZE, TARGET_STATE_QUEUE_SIZE
    );

    phase2_successful
}

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
    println!(
        "Pre-filled address queue: {} successful addresses in {:?}",
        successful, elapsed
    );
    println!(
        "Throughput: {:.2} addr/s",
        successful as f64 / elapsed.as_secs_f64()
    );

    successful
}

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

async fn perform_cross_tree_transfer<R: Rpc>(
    rpc: &mut R,
    env: &TestAccounts,
    payer: &Keypair,
    mint: &Pubkey,
    source_tree_index: usize,
    dest_tree_index: usize,
) -> Result<Signature, Box<dyn std::error::Error>> {
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

    let source_merkle_tree = env.v2_state_trees[source_tree_index].merkle_tree;
    input_compressed_accounts.retain(|acc| {
        acc.compressed_account.merkle_context.merkle_tree_pubkey == source_merkle_tree
    });

    if input_compressed_accounts.len() < MINT_TO_NUM as usize {
        return Err(format!(
            "Not enough tokens on tree[{}]: found {}, need {}.",
            source_tree_index,
            input_compressed_accounts.len(),
            MINT_TO_NUM
        )
        .into());
    }

    let rng = &mut rand::thread_rng();
    input_compressed_accounts.shuffle(rng);
    input_compressed_accounts.truncate(MINT_TO_NUM as usize);

    let total_tokens: u64 = input_compressed_accounts
        .iter()
        .map(|acc| acc.token_data.amount)
        .sum();

    let compressed_account_hashes: Vec<[u8; 32]> = input_compressed_accounts
        .iter()
        .map(|acc| acc.compressed_account.hash())
        .collect::<Result<Vec<_>, _>>()?;

    let proof_for_compressed_accounts = rpc
        .indexer()?
        .get_validity_proof(compressed_account_hashes, vec![], None)
        .await?;

    let root_indices = proof_for_compressed_accounts.value.get_root_indices();
    let merkle_contexts: Vec<_> = input_compressed_accounts
        .iter()
        .map(|acc| acc.compressed_account.merkle_context)
        .collect();

    let amount_per_output = total_tokens / MINT_TO_NUM;
    let compressed_accounts: Vec<TokenTransferOutputData> = (0..MINT_TO_NUM)
        .map(|_| TokenTransferOutputData {
            amount: amount_per_output,
            owner: payer.pubkey(),
            lamports: None,
            merkle_tree: env.v2_state_trees[dest_tree_index].output_queue,
        })
        .collect();

    let proof = proof_for_compressed_accounts
        .value
        .proof
        .0
        .map(|p| CompressedProof {
            a: p.a,
            b: p.b,
            c: p.c,
        });

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

async fn setup_rpc_connection(forester: &Keypair) -> LightClient {
    let mut rpc = LightClient::new(LightClientConfig::local()).await.unwrap();
    rpc.payer = forester.insecure_clone();
    rpc
}
async fn ensure_sufficient_balance(rpc: &mut LightClient, pubkey: &Pubkey, target_balance: u64) {
    if rpc.get_balance(pubkey).await.unwrap() < target_balance {
        rpc.airdrop_lamports(pubkey, target_balance).await.unwrap();
    }
}

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

async fn get_queue_pending_items<R: Rpc>(rpc: &mut R, env: &TestAccounts) -> (usize, usize, usize) {
    let state_output = get_output_queue_pending(rpc, &env.v2_state_trees[0].output_queue).await;
    let state_input = get_input_queue_pending(rpc, &env.v2_state_trees[0].merkle_tree).await;
    let address = get_address_queue_pending(rpc, &env.v2_address_trees[0]).await;

    (state_output, state_input, address)
}

async fn get_output_queue_pending<R: Rpc>(rpc: &mut R, queue_pubkey: &Pubkey) -> usize {
    match rpc.get_account(*queue_pubkey).await {
        Ok(Some(mut account)) => {
            if let Ok(output_queue) =
                BatchedQueueAccount::output_from_bytes(account.data.as_mut_slice())
            {
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

    // Initialize tracing subscriber to see forester logs
    // tracing_subscriber::fmt()
    //     .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    //     .with_test_writer()
    //     .try_init()
    //     .ok();

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

    let mint_keypair = Keypair::new();
    let mint_pubkey = create_mint_helper_with_keypair(&mut rpc, &payer, &mint_keypair).await;
    println!("Created mint: {}", mint_pubkey);

    let (_, _, pre_state_root) = get_initial_merkle_tree_state(
        &mut rpc,
        &env.v2_state_trees[0].merkle_tree,
        TreeType::StateV2,
    )
    .await;

    let (_, _, pre_address_root) =
        get_initial_merkle_tree_state(&mut rpc, &env.v2_address_trees[0], TreeType::AddressV2)
            .await;

    println!("\n=== Initial State ===");
    println!("State root: {:?}[..8]", &pre_state_root[..8]);
    println!("Address root: {:?}[..8]", &pre_address_root[..8]);

    // Debug epoch state
    let protocol_config = get_protocol_config(&mut rpc).await;
    let current_slot = rpc.get_slot().await.unwrap();
    let current_epoch = protocol_config.get_current_epoch(current_slot);
    let phases = get_epoch_phases(&protocol_config, current_epoch);

    println!("\n=== Epoch Debug Info ===");
    println!("Protocol config:");
    println!("  genesis_slot: {}", protocol_config.genesis_slot);
    println!(
        "  active_phase_length: {}",
        protocol_config.active_phase_length
    );
    println!(
        "  registration_phase_length: {}",
        protocol_config.registration_phase_length
    );
    println!(
        "  report_work_phase_length: {}",
        protocol_config.report_work_phase_length
    );
    println!("Current slot: {}", current_slot);
    println!("Current epoch: {}", current_epoch);
    println!("Epoch {} phases:", current_epoch);
    println!(
        "  registration: {} - {}",
        phases.registration.start, phases.registration.end
    );
    println!("  active: {} - {}", phases.active.start, phases.active.end);
    println!(
        "  report_work: {} - {}",
        phases.report_work.start, phases.report_work.end
    );

    // Check which phase we're in
    if current_slot < phases.registration.end {
        println!("STATUS: In REGISTRATION phase (can register)");
    } else if current_slot < phases.active.start {
        println!(
            "STATUS: Between registration and active (WAITING for active phase at slot {})",
            phases.active.start
        );
    } else if current_slot < phases.active.end {
        println!("STATUS: In ACTIVE phase (should be processing)");
    } else {
        println!("STATUS: Past active phase");
    }

    // PRE-FILL QUEUES BEFORE STARTING FORESTER
    println!("\n=== PHASE 1: PRE-FILLING QUEUES ===");

    let state_txs =
        prefill_state_queue(&mut rpc, &env, &payer, &mint_pubkey, NUM_STATE_TRANSACTIONS).await;

    let address_txs = prefill_address_queue(&mut rpc, &env, &payer, NUM_ADDRESS_TRANSACTIONS).await;

    let (state_out, state_in, addr_queue) = get_queue_pending_items(&mut rpc, &env).await;
    println!("\n=== Queue Status Before Forester ===");
    println!("State output queue: {} pending items", state_out);
    println!("State input queue: {} pending items", state_in);
    println!("Address queue: {} pending items", addr_queue);

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
    println!(
        "Queues are properly filled (state_output: {}/{}, address: {}/{}, total: {})",
        state_out,
        TARGET_STATE_QUEUE_SIZE,
        addr_queue,
        TARGET_ADDRESS_QUEUE_SIZE,
        total_queue_items
    );

    println!("\n=== PHASE 2: STARTING FORESTER ===");

    let config = ForesterConfig {
        external_services: ExternalServicesConfig {
            rpc_url: get_rpc_url(),
            ws_rpc_url: Some(get_ws_rpc_url()),
            indexer_url: Some(get_indexer_url()),
            prover_url: Some(get_prover_url()),
            prover_append_url: Some(get_prover_url()),
            prover_update_url: Some(get_prover_url()),
            prover_address_append_url: Some(get_prover_url()),
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
            skip_v1_state_trees: true,
            skip_v2_state_trees: false,
            skip_v1_address_trees: true,
            skip_v2_address_trees: false,
            tree_ids: vec![
                env.v2_state_trees[0].merkle_tree,
                env.v2_state_trees[1].merkle_tree,
                env.v2_state_trees[2].merkle_tree,
                env.v2_state_trees[3].merkle_tree,
                env.v2_state_trees[4].merkle_tree,
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

    let active_slot = get_next_active_phase_with_time(&mut rpc, &protocol_config, 100).await;
    let current_slot = rpc.get_slot().await.unwrap();
    println!(
        "Current slot: {}, Active phase starts at slot {}",
        current_slot, active_slot
    );

    println!(
        "Waiting until slot {} to start forester...",
        active_slot.saturating_sub(5)
    );
    wait_for_slot(&mut rpc, active_slot.saturating_sub(5)).await;

    let (
        service_handle,
        shutdown_sender,
        shutdown_compressible_sender,
        shutdown_bootstrap_sender,
        mut work_report_receiver,
    ) = setup_forester_pipeline(&config).await;

    // Check if the forester task failed early
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    if service_handle.is_finished() {
        match service_handle.await {
            Ok(Ok(())) => panic!("Forester finished successfully but unexpectedly early"),
            Ok(Err(e)) => panic!("Forester failed with error: {:?}", e),
            Err(e) => panic!("Forester task panicked: {:?}", e),
        }
    }
    println!("Forester pipeline started successfully, waiting for work reports...");

    println!("Forester pipeline started, waiting for first work report...");
    // Wait for active phase
    wait_for_slot(&mut rpc, active_slot).await;
    println!("Active phase started at slot {}", active_slot);

    // Monitor work reports - use ProcessingMetrics from reports
    let mut total_processed = 0;
    let mut total_metrics = ProcessingMetrics::default();
    let mut last_report_time = Instant::now();
    let mut epochs_completed = 0;
    const MAX_EPOCHS: usize = 10;

    let timeout_duration = Duration::from_secs(DEFAULT_TIMEOUT_SECONDS);
    let _result = timeout(timeout_duration, async {
        while let Some(report) = work_report_receiver.recv().await {
            epochs_completed += 1;
            println!(
                "Work report: epoch={} items={} total={:?} (epoch {}/{})",
                report.epoch,
                report.processed_items,
                report.metrics.total(),
                epochs_completed,
                MAX_EPOCHS
            );
            println!(
                "  Append:  circuit={:?}, proof={:?}, round_trip={:?}",
                report.metrics.append.circuit_inputs_duration,
                report.metrics.append.proof_generation_duration,
                report.metrics.append.round_trip_duration
            );
            println!(
                "  Nullify: circuit={:?}, proof={:?}, round_trip={:?}",
                report.metrics.nullify.circuit_inputs_duration,
                report.metrics.nullify.proof_generation_duration,
                report.metrics.nullify.round_trip_duration
            );
            println!(
                "  Address: circuit={:?}, proof={:?}, round_trip={:?}",
                report.metrics.address_append.circuit_inputs_duration,
                report.metrics.address_append.proof_generation_duration,
                report.metrics.address_append.round_trip_duration
            );

            if report.processed_items > 0 {
                total_processed = total_processed.max(report.processed_items);
                total_metrics += report.metrics;
            }

            if last_report_time.elapsed() > Duration::from_secs(5) {
                println!(
                    "Progress: {} items in {:?}",
                    total_processed,
                    total_metrics.total()
                );
                last_report_time = Instant::now();
            }

            // Check queue status after each epoch
            let (state_out, state_in, addr_queue) = get_queue_pending_items(&mut rpc, &env).await;
            println!(
                "  Queue status: state_out={}, state_in={}, addr={}",
                state_out, state_in, addr_queue
            );

            // Exit early if all queues are empty
            if state_out == 0 && state_in == 0 && addr_queue == 0 {
                println!("\nAll queues empty after {} epochs!", epochs_completed);
                break;
            }

            // Don't assert until we've given it MAX_EPOCHS chances
            if epochs_completed >= MAX_EPOCHS {
                println!("\nReached {} epochs, checking final state...", MAX_EPOCHS);
                break;
            }
        }
        Ok::<(), ()>(())
    })
    .await;

    let processing_time = total_metrics.total();

    let state_output_items = TARGET_STATE_QUEUE_SIZE;
    let state_input_items = TARGET_STATE_QUEUE_SIZE;
    let address_items = TARGET_ADDRESS_QUEUE_SIZE;

    println!("\n=== PERFORMANCE TEST RESULTS ===");
    println!("Total processing time: {:?}", processing_time);
    println!("\n--- Phase Breakdown by Circuit Type ---");
    println!("  Append (state output queue):");
    println!(
        "    Circuit inputs: {:?}",
        total_metrics.append.circuit_inputs_duration
    );
    println!(
        "    Proof gen:      {:?}",
        total_metrics.append.proof_generation_duration
    );
    println!(
        "    Round trip:     {:?} (cumulative)",
        total_metrics.append.round_trip_duration
    );
    println!("    Total:          {:?}", total_metrics.append.total());
    println!("  Nullify (state input queue):");
    println!(
        "    Circuit inputs: {:?}",
        total_metrics.nullify.circuit_inputs_duration
    );
    println!(
        "    Proof gen:      {:?}",
        total_metrics.nullify.proof_generation_duration
    );
    println!(
        "    Round trip:     {:?} (cumulative)",
        total_metrics.nullify.round_trip_duration
    );
    println!("    Total:          {:?}", total_metrics.nullify.total());
    println!("  AddressAppend:");
    println!(
        "    Circuit inputs: {:?}",
        total_metrics.address_append.circuit_inputs_duration
    );
    println!(
        "    Proof gen:      {:?}",
        total_metrics.address_append.proof_generation_duration
    );
    println!(
        "    Round trip:     {:?} (cumulative)",
        total_metrics.address_append.round_trip_duration
    );
    println!(
        "    Total:          {:?}",
        total_metrics.address_append.total()
    );
    println!("\n--- Totals ---");
    println!(
        "  Total circuit inputs: {:?} ({:.1}%)",
        total_metrics.total_circuit_inputs(),
        100.0 * total_metrics.total_circuit_inputs().as_secs_f64()
            / processing_time.as_secs_f64().max(0.001)
    );
    println!(
        "  Total proof gen:      {:?} ({:.1}%)",
        total_metrics.total_proof_generation(),
        100.0 * total_metrics.total_proof_generation().as_secs_f64()
            / processing_time.as_secs_f64().max(0.001)
    );
    println!(
        "  Total round trip:     {:?} (cumulative, proofs run in parallel)",
        total_metrics.total_round_trip()
    );
    let parallelism = total_metrics.total_round_trip().as_secs_f64()
        / processing_time.as_secs_f64().max(0.001);
    println!(
        "  Effective parallelism: {:.1}x (round_trip / wall_clock)",
        parallelism
    );
    println!("\nTotal items processed: {}", total_processed);
    println!("  State output items: {}", state_output_items);
    println!("  State input items: {}", state_input_items);
    println!("  Address items: {}", address_items);
    println!("\nThroughput (items/second, based on total processing time):");
    let processing_secs = processing_time.as_secs_f64();
    if processing_secs > 0.0 {
        println!(
            "  Overall: {:.2} items/second",
            total_processed as f64 / processing_secs
        );
        println!(
            "  State output: {:.2} items/second",
            state_output_items as f64 / processing_secs
        );
        println!(
            "  State input: {:.2} items/second",
            state_input_items as f64 / processing_secs
        );
        println!(
            "  Address: {:.2} items/second",
            address_items as f64 / processing_secs
        );
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

    let (_, _, post_address_root) =
        get_initial_merkle_tree_state(&mut rpc, &env.v2_address_trees[0], TreeType::AddressV2)
            .await;

    println!("\n=== Root Verification ===");
    println!("State root changed: {}", pre_state_root != post_state_root);
    println!("  Before: {:?}[..8]", &pre_state_root[..8]);
    println!("  After:  {:?}[..8]", &post_state_root[..8]);
    println!(
        "Address root changed: {}",
        pre_address_root != post_address_root
    );
    println!("  Before: {:?}[..8]", &pre_address_root[..8]);
    println!("  After:  {:?}[..8]", &post_address_root[..8]);

    assert_ne!(
        pre_state_root, post_state_root,
        "State root should have changed after forester processing"
    );
    assert_ne!(
        pre_address_root, post_address_root,
        "Address root should have changed after forester processing"
    );
    println!("Roots changed correctly");

    // Verify queues are now empty (or nearly empty)
    let (final_state_out, final_state_in, final_addr_queue) =
        get_queue_pending_items(&mut rpc, &env).await;
    println!("\n=== Queue Status After Forester ===");
    println!("State output queue: {} pending items", final_state_out);
    println!("State input queue: {} pending items", final_state_in);
    println!("Address queue: {} pending items", final_addr_queue);

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

    println!(
        "All queues are empty or nearly empty (state_output: {}, state_input: {}, address: {})",
        final_state_out, final_state_in, final_addr_queue
    );

    // Cleanup
    shutdown_sender.send(()).unwrap();
    shutdown_compressible_sender.send(()).unwrap();
    shutdown_bootstrap_sender.send(()).unwrap();
    service_handle.await.unwrap().unwrap();

    println!("\nPerformance test completed successfully!");
}
