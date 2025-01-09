use std::{sync::Arc, time::Duration};

use forester::run_pipeline;
use forester_utils::registry::{register_test_forester, update_test_forester};
use light_batched_merkle_tree::{
    batch::BatchState, initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_client::{
    rpc::{solana_rpc::SolanaRpcUrl, RpcConnection, SolanaRpcConnection},
    rpc_pool::SolanaRpcPool,
};
use light_program_test::test_env::EnvAccounts;
use light_prover_client::gnark::helpers::LightValidatorConfig;
use light_test_utils::{
    e2e_test_env::{init_program_test_env, E2ETestEnv},
    indexer::TestIndexer,
};
use serial_test::serial;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer,
};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    time::timeout,
};
use tracing::log::info;

use crate::test_utils::{forester_config, init};

mod test_utils;

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
async fn test_state_batched() {
    let devnet = false;
    let tree_params = if devnet {
        InitStateTreeAccountsInstructionData::default()
    } else {
        InitStateTreeAccountsInstructionData::test_default()
    };

    init(Some(LightValidatorConfig {
        enable_indexer: false,
        wait_time: 10,
        prover_config: None,
        sbf_programs: vec![],
    }))
    .await;

    let forester_keypair = Keypair::new();
    let mut env = EnvAccounts::get_local_test_validator_accounts();
    env.forester = forester_keypair.insecure_clone();

    let mut config = forester_config();
    config.payer_keypair = forester_keypair.insecure_clone();

    let pool = SolanaRpcPool::<SolanaRpcConnection>::new(
        config.external_services.rpc_url.to_string(),
        CommitmentConfig::processed(),
        config.general_config.rpc_pool_size as u32,
    )
    .await
    .unwrap();

    let commitment_config = CommitmentConfig::confirmed();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, Some(commitment_config));
    rpc.payer = forester_keypair.insecure_clone();

    rpc.airdrop_lamports(&forester_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    rpc.airdrop_lamports(
        &env.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100_000,
    )
    .await
    .unwrap();

    register_test_forester(
        &mut rpc,
        &env.governance_authority,
        &forester_keypair.pubkey(),
        light_registry::ForesterConfig::default(),
    )
    .await
    .unwrap();

    let new_forester_keypair = Keypair::new();
    rpc.airdrop_lamports(&new_forester_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    update_test_forester(
        &mut rpc,
        &forester_keypair,
        &forester_keypair.pubkey(),
        Some(&new_forester_keypair),
        light_registry::ForesterConfig::default(),
    )
    .await
    .unwrap();

    config.derivation_pubkey = forester_keypair.pubkey();
    config.payer_keypair = new_forester_keypair.insecure_clone();

    let merkle_tree_keypair = Keypair::new();
    let nullifier_queue_keypair = Keypair::new();
    let cpi_context_keypair = Keypair::new();

    let mut e2e_env: E2ETestEnv<SolanaRpcConnection, TestIndexer<SolanaRpcConnection>>;

    e2e_env = init_program_test_env(rpc, &env, false).await;
    e2e_env.indexer.state_merkle_trees.clear();
    e2e_env
        .indexer
        .add_state_merkle_tree(
            &mut e2e_env.rpc,
            &merkle_tree_keypair,
            &nullifier_queue_keypair,
            &cpi_context_keypair,
            None,
            None,
            2,
        )
        .await;
    let state_merkle_tree_pubkey = e2e_env.indexer.state_merkle_trees[0].accounts.merkle_tree;
    let mut merkle_tree_account = e2e_env
        .rpc
        .get_account(state_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    let merkle_tree =
        BatchedMerkleTreeAccount::state_tree_from_bytes_mut(&mut merkle_tree_account.data).unwrap();

    let (initial_next_index, initial_sequence_number, pre_root) = {
        let mut rpc = pool.get_connection().await.unwrap();
        let mut merkle_tree_account = rpc
            .get_account(merkle_tree_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();

        let merkle_tree = BatchedMerkleTreeAccount::state_tree_from_bytes_mut(
            merkle_tree_account.data.as_mut_slice(),
        )
        .unwrap();

        let initial_next_index = merkle_tree.get_metadata().next_index;
        let initial_sequence_number = merkle_tree.get_metadata().sequence_number;

        (
            initial_next_index,
            initial_sequence_number,
            merkle_tree.get_root().unwrap(),
        )
    };

    info!(
        "Initial state:
        next_index: {}
        sequence_number: {}
        batch_size: {}",
        initial_next_index,
        initial_sequence_number,
        merkle_tree.get_metadata().queue_metadata.batch_size
    );

    for i in 0..merkle_tree.get_metadata().queue_metadata.batch_size {
        println!("\ntx {}", i);

        e2e_env
            .compress_sol_deterministic(&forester_keypair, 1_000_000, None)
            .await;
        e2e_env
            .transfer_sol_deterministic(&forester_keypair, &Pubkey::new_unique(), None)
            .await
            .unwrap();
    }
    let (state_merkle_tree_bundle, _, _) = (
        e2e_env.indexer.state_merkle_trees[0].clone(),
        e2e_env.indexer.address_merkle_trees[0].clone(),
        e2e_env.rpc,
    );

    println!(
        "state merkle tree pub key: {}",
        state_merkle_tree_bundle.accounts.merkle_tree
    );
    println!(
        "output queue pub key: {}",
        state_merkle_tree_bundle.accounts.nullifier_queue
    );

    println!("data appended");

    let num_output_zkp_batches =
        tree_params.input_queue_batch_size / tree_params.output_queue_zkp_batch_size;

    println!("num_output_zkp_batches: {}", num_output_zkp_batches);

    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (work_report_sender, mut work_report_receiver) = mpsc::channel(100);

    let service_handle = tokio::spawn(run_pipeline(
        Arc::from(config.clone()),
        Arc::new(Mutex::new(e2e_env.indexer)),
        shutdown_receiver,
        work_report_sender,
    ));

    let timeout_duration = Duration::from_secs(60 * 10);
    match timeout(timeout_duration, work_report_receiver.recv()).await {
        Ok(Some(report)) => {
            info!("Received work report: {:?}", report);
            info!(
                "Work report debug:
                reported_items: {}
                batch_size: {}
                complete_batches: {}",
                report.processed_items,
                tree_params.input_queue_batch_size,
                report.processed_items / tree_params.input_queue_batch_size as usize,
            );
            assert!(report.processed_items > 0, "No items were processed");

            let batch_size = tree_params.input_queue_batch_size;
            assert_eq!(
                report.processed_items % batch_size as usize,
                0,
                "Processed items {} should be a multiple of batch size {}",
                report.processed_items,
                batch_size
            );
        }
        Ok(None) => panic!("Work report channel closed unexpectedly"),
        Err(_) => panic!("Test timed out after {:?}", timeout_duration),
    }

    let mut rpc = pool.get_connection().await.unwrap();
    let mut merkle_tree_account = rpc
        .get_account(merkle_tree_keypair.pubkey())
        .await
        .unwrap()
        .unwrap();

    let merkle_tree = BatchedMerkleTreeAccount::state_tree_from_bytes_mut(
        merkle_tree_account.data.as_mut_slice(),
    )
    .unwrap();

    assert!(
        merkle_tree
            .get_metadata()
            .queue_metadata
            .next_full_batch_index
            > 0,
        "No batches were processed"
    );

    {
        let mut rpc = pool.get_connection().await.unwrap();

        let mut merkle_tree_account = rpc
            .get_account(merkle_tree_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();

        let merkle_tree = BatchedMerkleTreeAccount::state_tree_from_bytes_mut(
            merkle_tree_account.data.as_mut_slice(),
        )
        .unwrap();

        let final_metadata = merkle_tree.get_metadata();

        let mut output_queue_account = rpc
            .get_account(nullifier_queue_keypair.pubkey())
            .await
            .unwrap()
            .unwrap();

        let output_queue = BatchedQueueAccount::output_queue_from_bytes_mut(
            output_queue_account.data.as_mut_slice(),
        )
        .unwrap();

        let batch_size = merkle_tree.get_metadata().queue_metadata.batch_size;
        let zkp_batch_size = merkle_tree.get_metadata().queue_metadata.zkp_batch_size;
        let num_zkp_batches = batch_size / zkp_batch_size;

        let mut completed_items = 0;
        for batch_idx in 0..output_queue.batches.len() {
            let batch = output_queue.batches.get(batch_idx).unwrap();
            if batch.get_state() == BatchState::Inserted {
                completed_items += batch_size;
            }
        }
        info!(
            "initial_next_index: {}
            final_next_index: {}
            batch_size: {}
            zkp_batch_size: {}
            num_zkp_batches per full batch: {}
            completed_items from batch states: {}
            input_queue_metadata: {:?}
            output_queue_metadata: {:?}",
            initial_next_index,
            final_metadata.next_index,
            batch_size,
            zkp_batch_size,
            num_zkp_batches,
            completed_items,
            final_metadata.queue_metadata,
            output_queue.get_metadata().batch_metadata
        );

        assert_eq!(
            final_metadata.next_index,
            initial_next_index + completed_items,
            "Merkle tree next_index did not advance by expected amount",
        );

        assert_eq!(
            merkle_tree
                .get_metadata()
                .queue_metadata
                .next_full_batch_index,
            1
        );

        assert!(
            final_metadata.sequence_number > initial_sequence_number,
            "Sequence number should have increased"
        );

        // compress_sol_deterministic creates 1 output
        // transfer_sol_deterministic invalidates 1 input and creates 1 output
        // 1 + 1 + 1 = 3
        const UPDATES_PER_BATCH: u64 = 3;

        let expected_sequence_number =
            initial_sequence_number + (num_zkp_batches * UPDATES_PER_BATCH);
        let expected_root_history_len = (expected_sequence_number + 1) as usize;

        assert_eq!(final_metadata.sequence_number, expected_sequence_number);

        assert_eq!(merkle_tree.root_history.len(), expected_root_history_len);

        let post_root = merkle_tree.get_root().unwrap();
        assert_ne!(pre_root, post_root, "Roots are the same");

        assert_ne!(
            pre_root,
            merkle_tree.get_root().unwrap(),
            "Root should have changed"
        );
    }

    shutdown_sender
        .send(())
        .expect("Failed to send shutdown signal");
    service_handle.await.unwrap().unwrap();
}
