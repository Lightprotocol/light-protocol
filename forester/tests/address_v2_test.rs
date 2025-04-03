use std::{sync::Arc, time::Duration};

use forester::{epoch_manager::WorkReport, run_pipeline, ForesterConfig};
use forester_utils::{forester_epoch::get_epoch_phases, utils::wait_for_indexer};
use light_batched_merkle_tree::{
    initialize_address_tree::InitAddressTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount,
};
use light_client::{
    indexer::{photon_indexer::PhotonIndexer, AddressWithTree, Indexer},
    rpc::{
        merkle_tree::MerkleTreeExt, solana_rpc::SolanaRpcUrl, RpcConnection, SolanaRpcConnection,
    },
};
use light_compressed_account::{
    address::derive_address,
    instruction_data::{compressed_proof::CompressedProof, data::NewAddressParams},
};
use light_program_test::{indexer::TestIndexer, test_env::EnvAccounts};
use light_prover_client::gnark::helpers::{LightValidatorConfig, ProverConfig, ProverMode};
use light_registry::{
    protocol_config::state::ProtocolConfig, utils::get_protocol_config_pda_address,
};
use light_test_utils::create_address_test_program_sdk::{
    create_pda_instruction, CreateCompressedPdaInstructionInputs,
};
use rand::{prelude::StdRng, Rng, SeedableRng};
use serial_test::serial;
use solana_program::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer};
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::test_utils::{forester_config, init};

mod test_utils;

const PHOTON_INDEXER_URL: &str = "http://127.0.0.1:8784";
const DEFAULT_TIMEOUT_SECONDS: u64 = 120;
const COMPUTE_BUDGET_LIMIT: u32 = 1_000_000;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[serial]
async fn test_create_v2_address() {
    let seed = 0;
    println!("\n\ne2e test seed {}\n\n", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    let tree_params = InitAddressTreeAccountsInstructionData::test_default();

    init(Some(LightValidatorConfig {
        enable_indexer: true,
        wait_time: 10,
        prover_config: Some(ProverConfig {
            run_mode: Some(ProverMode::ForesterTest),
            circuits: vec![],
        }),
        sbf_programs: vec![(
            "FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy".to_string(),
            "../target/deploy/create_address_test_program.so".to_string(),
        )],
        limit_ledger_size: Some(500000),
    }))
    .await;

    let env = EnvAccounts::get_local_test_validator_accounts();
    let mut config = forester_config();
    config.transaction_config.batch_ixs_per_tx = 1;
    config.payer_keypair = env.forester.insecure_clone();
    config.derivation_pubkey = env.forester.pubkey();

    let mut rpc =
        SolanaRpcConnection::new(SolanaRpcUrl::Localnet, Some(CommitmentConfig::processed()));
    rpc.payer = env.forester.insecure_clone();

    ensure_sufficient_balance(&mut rpc, &env.forester.pubkey(), LAMPORTS_PER_SOL * 100).await;

    let mut photon_indexer = PhotonIndexer::new(
        PHOTON_INDEXER_URL.to_string(),
        None,
        SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None),
    );

    let (_, _, pre_root) =
        get_initial_merkle_tree_state(&mut rpc, &env.batch_address_merkle_tree).await;

    let batch_payer = Keypair::from_bytes(&[
        88, 117, 248, 40, 40, 5, 251, 124, 235, 221, 10, 212, 169, 203, 91, 203, 255, 67, 210, 150,
        87, 182, 238, 155, 87, 24, 176, 252, 157, 119, 68, 81, 148, 156, 30, 0, 60, 63, 34, 247,
        192, 120, 4, 170, 32, 149, 221, 144, 74, 244, 181, 142, 37, 197, 196, 136, 159, 196, 101,
        21, 194, 56, 163, 1,
    ])
    .unwrap();
    ensure_sufficient_balance(&mut rpc, &batch_payer.pubkey(), LAMPORTS_PER_SOL * 100).await;

    let batch_size = get_batch_size(&mut rpc, &env.batch_address_merkle_tree).await;

    for i in 0..batch_size {
        println!("====== Creating v2 address {} ======", i);
        let result = create_v2_address(
            &mut rpc,
            &mut photon_indexer,
            &env.batch_address_merkle_tree,
            &env.registered_program_pda,
            &batch_payer,
            &env,
            &mut rng,
        )
        .await;

        println!("====== result: {:?} ======", result);
    }

    let mut address_tree_account = rpc
        .get_account(env.batch_address_merkle_tree)
        .await
        .unwrap()
        .unwrap();

    let address_tree = BatchedMerkleTreeAccount::address_from_bytes(
        address_tree_account.data.as_mut_slice(),
        &env.batch_address_merkle_tree.into(),
    )
    .unwrap();

    println!("Address tree metadata: {:?}", address_tree.get_metadata());

    let protocol_config = get_protocol_config(&mut rpc).await;
    let _active_phase_slot = get_active_phase_start_slot(&mut rpc, &protocol_config).await;

    let (service_handle, shutdown_sender, mut work_report_receiver) =
        setup_forester_pipeline(&config).await;

    wait_for_work_report(&mut work_report_receiver, &tree_params).await;

    verify_root_changed(&mut rpc, &env.batch_address_merkle_tree, &pre_root).await;

    shutdown_sender
        .send(())
        .expect("Failed to send shutdown signal");
    service_handle.await.unwrap().unwrap();
}

async fn ensure_sufficient_balance(
    rpc: &mut SolanaRpcConnection,
    pubkey: &Pubkey,
    target_balance: u64,
) {
    if rpc.get_balance(pubkey).await.unwrap() < target_balance {
        rpc.airdrop_lamports(pubkey, target_balance).await.unwrap();
    }
}

async fn get_protocol_config(rpc: &mut SolanaRpcConnection) -> ProtocolConfig {
    let protocol_config_pda_address = get_protocol_config_pda_address().0;
    rpc.get_anchor_account::<light_registry::protocol_config::state::ProtocolConfigPda>(
        &protocol_config_pda_address,
    )
    .await
    .unwrap()
    .unwrap()
    .config
}

async fn setup_forester_pipeline(
    config: &ForesterConfig,
) -> (
    tokio::task::JoinHandle<anyhow::Result<()>>,
    oneshot::Sender<()>,
    mpsc::Receiver<WorkReport>,
) {
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (work_report_sender, work_report_receiver) = mpsc::channel(100);

    let rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    let forester_photon_indexer = PhotonIndexer::new(PHOTON_INDEXER_URL.to_string(), None, rpc);

    let service_handle = tokio::spawn(run_pipeline(
        Arc::from(config.clone()),
        None,
        None,
        Arc::new(Mutex::new(forester_photon_indexer)),
        shutdown_receiver,
        work_report_sender,
    ));

    (service_handle, shutdown_sender, work_report_receiver)
}

async fn wait_for_work_report(
    work_report_receiver: &mut mpsc::Receiver<WorkReport>,
    tree_params: &InitAddressTreeAccountsInstructionData,
) {
    let batch_size = tree_params.input_queue_batch_size as usize;
    let minimum_processed_items: usize = tree_params.input_queue_batch_size as usize;
    let mut total_processed_items: usize = 0;
    let timeout_duration = Duration::from_secs(DEFAULT_TIMEOUT_SECONDS);

    println!("Waiting for work reports...");
    println!("Batch size: {}", batch_size);
    println!(
        "Minimum required processed items: {}",
        minimum_processed_items
    );

    let start_time = tokio::time::Instant::now();
    while total_processed_items < minimum_processed_items {
        match tokio::time::timeout(
            timeout_duration.saturating_sub(start_time.elapsed()),
            work_report_receiver.recv(),
        )
        .await
        {
            Ok(Some(report)) => {
                println!("Received work report: {:?}", report);
                total_processed_items += report.processed_items;
            }
            Ok(None) => {
                println!("Work report channel closed unexpectedly");
                break;
            }
            Err(_) => {
                println!("Timed out after waiting for {:?}", timeout_duration);
                break;
            }
        }
    }

    println!("Total processed items: {}", total_processed_items);
    assert!(
        total_processed_items >= minimum_processed_items,
        "Processed fewer items ({}) than required ({})",
        total_processed_items,
        minimum_processed_items
    );
}

async fn get_active_phase_start_slot<R: RpcConnection>(
    rpc: &mut R,
    protocol_config: &ProtocolConfig,
) -> u64 {
    let current_slot = rpc.get_slot().await.unwrap();
    let current_epoch = protocol_config.get_current_epoch(current_slot);
    let phases = get_epoch_phases(protocol_config, current_epoch);
    phases.active.start
}

async fn create_v2_address<R: RpcConnection + MerkleTreeExt, I: Indexer<R>>(
    rpc: &mut R,
    indexer: &mut I,
    batch_address_merkle_tree: &Pubkey,
    registered_program_pda: &Pubkey,
    payer: &Keypair,
    env: &EnvAccounts,
    rng: &mut StdRng,
) -> Result<(), light_client::rpc::RpcError> {
    let data: [u8; 31] = [1; 31];

    let seed = rng.gen();
    let address = derive_address(
        &seed,
        &batch_address_merkle_tree.to_bytes(),
        &create_address_test_program::ID.to_bytes(),
    );

    println!("Creating v2 address with:");
    println!("- address: {:?}", address);
    println!(
        "- address_merkle_tree_pubkey: {:?}",
        batch_address_merkle_tree
    );
    println!("- program_id: {:?}", create_address_test_program::ID);
    println!("- seed: {:?}", seed);

    wait_for_indexer(rpc, indexer).await.unwrap();

    let address_with_tree = AddressWithTree {
        address,
        tree: *batch_address_merkle_tree,
    };

    let mut test_indexer: TestIndexer<R> = TestIndexer::init_from_env(payer, env, None).await;
    let test_rpc_result = test_indexer
        .create_proof_for_compressed_accounts(
            None,
            None,
            Some(&[address]),
            Some(vec![env.batch_address_merkle_tree]),
            rpc,
        )
        .await
        .unwrap();
    println!("test_indexer result: {:?}", test_rpc_result);

    let photon_rpc_result = indexer
        .get_validity_proof_v2(vec![], vec![address_with_tree])
        .await
        .unwrap();

    println!("photon result: {:?}", photon_rpc_result);

    let new_address_params = NewAddressParams {
        seed,
        address_merkle_tree_pubkey: *batch_address_merkle_tree,
        address_queue_pubkey: *batch_address_merkle_tree,
        address_merkle_tree_root_index: test_rpc_result.address_root_indices[0], // photon_rpc_result.root_indices[0].root_index,
    };

    let proof = test_rpc_result.proof; // photon_rpc_result.compressed_proof.unwrap();
    let proof = CompressedProof {
        a: proof.a,
        b: proof.b,
        c: proof.c,
    };

    let create_ix_inputs = CreateCompressedPdaInstructionInputs {
        data,
        signer: &payer.pubkey(),
        output_compressed_account_merkle_tree_pubkey: &env.merkle_tree_pubkey,
        proof: &proof,
        new_address_params,
        registered_program_pda,
    };

    let instruction = create_pda_instruction(create_ix_inputs);

    let instructions = vec![
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            COMPUTE_BUDGET_LIMIT,
        ),
        instruction,
    ];

    let result = rpc
        .create_and_send_transaction(&instructions, &payer.pubkey(), &[payer])
        .await;

    println!("result: {:?}", result);
    Ok(())
}

async fn get_batch_size<R: RpcConnection>(rpc: &mut R, merkle_tree_pubkey: &Pubkey) -> u64 {
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &merkle_tree_pubkey.into(),
    )
    .unwrap();

    merkle_tree.get_metadata().queue_batches.batch_size
}

async fn get_initial_merkle_tree_state(
    rpc: &mut SolanaRpcConnection,
    merkle_tree_pubkey: &Pubkey,
) -> (u64, u64, [u8; 32]) {
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
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

async fn verify_root_changed(
    rpc: &mut SolanaRpcConnection,
    merkle_tree_pubkey: &Pubkey,
    pre_root: &[u8; 32],
) {
    let mut merkle_tree_account = rpc.get_account(*merkle_tree_pubkey).await.unwrap().unwrap();
    let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &merkle_tree_pubkey.into(),
    )
    .unwrap();
    println!("merkle tree metadata: {:?}", merkle_tree.get_metadata());
    assert_ne!(
        *pre_root,
        merkle_tree.get_root().unwrap(),
        "Root should have changed"
    );
}
