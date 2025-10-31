use std::{sync::Arc, time::Duration};

use forester::{config::GeneralConfig, run_pipeline};
use forester_utils::{
    registry::{register_test_forester, update_test_forester},
    rpc_pool::SolanaRpcPoolBuilder,
};
use light_batched_merkle_tree::{
    batch::BatchState, initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_client::{
    local_test_validator::LightValidatorConfig,
    rpc::{client::RpcUrl, LightClient, LightClientConfig, Rpc},
};
use light_compressed_account::TreeType;
use light_program_test::{accounts::test_accounts::TestAccounts, indexer::TestIndexer};
use light_test_utils::e2e_test_env::{init_program_test_env, E2ETestEnv};
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
        enable_prover: true,
        wait_time: 30,
        sbf_programs: vec![],
        limit_ledger_size: None,
        grpc_port: None,
    }))
    .await;

    let forester_keypair = Keypair::new();
    let mut env = TestAccounts::get_local_test_validator_accounts();
    env.protocol.forester = forester_keypair.insecure_clone();

    let mut config = forester_config();
    config.payer_keypair = forester_keypair.insecure_clone();
    config.general_config = GeneralConfig::test_state_v2();

    let pool = SolanaRpcPoolBuilder::<LightClient>::default()
        .url(config.external_services.rpc_url.to_string())
        .commitment(CommitmentConfig::processed())
        .build()
        .await
        .unwrap();

    let commitment_config = CommitmentConfig::confirmed();
    let mut rpc = LightClient::new(LightClientConfig {
        url: RpcUrl::Localnet.to_string(),
        photon_url: None,
        commitment_config: Some(commitment_config),
        fetch_active_tree: false,
    })
    .await
    .unwrap();
    rpc.payer = forester_keypair.insecure_clone();

    rpc.airdrop_lamports(&forester_keypair.pubkey(), LAMPORTS_PER_SOL * 100_000)
        .await
        .unwrap();

    rpc.airdrop_lamports(
        &env.protocol.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100_000,
    )
    .await
    .unwrap();

    register_test_forester(
        &mut rpc,
        &env.protocol.governance_authority,
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

    let mut e2e_env: E2ETestEnv<LightClient, TestIndexer> =
        init_program_test_env(rpc, &env, tree_params.output_queue_batch_size as usize).await;

    for tree in e2e_env.indexer.state_merkle_trees.iter() {
        println!("====================");
        println!("state merkle tree pub key: {}", tree.accounts.merkle_tree);
        println!("output queue pub key: {}", tree.accounts.nullifier_queue);
        println!("tree type: {}", tree.tree_type);
    }

    let (batched_state_merkle_tree_index, batched_state_merkle_tree_pubkey, nullifier_queue_pubkey) =
        e2e_env
            .indexer
            .state_merkle_trees
            .iter()
            .enumerate()
            .find(|(_, tree)| tree.tree_type == TreeType::StateV2)
            .map(|(index, tree)| {
                (
                    index,
                    tree.accounts.merkle_tree,
                    tree.accounts.nullifier_queue,
                )
            })
            .unwrap();

    // TODO: regenerate batched state merkle tree with rollover_fee = 1
    e2e_env.indexer.state_merkle_trees[batched_state_merkle_tree_index].rollover_fee = 1;

    let mut merkle_tree_account = e2e_env
        .rpc
        .get_account(batched_state_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
        &mut merkle_tree_account.data,
        &batched_state_merkle_tree_pubkey.into(),
    )
    .unwrap();

    let (initial_next_index, initial_sequence_number, pre_root) = {
        let rpc = pool.get_connection().await.unwrap();
        let mut merkle_tree_account = rpc
            .get_account(batched_state_merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap();

        let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &batched_state_merkle_tree_pubkey.into(),
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
        merkle_tree.get_metadata().queue_batches.batch_size
    );

    for i in 0..merkle_tree.get_metadata().queue_batches.batch_size {
        println!("\ntx {}", i);

        e2e_env
            .compress_sol_deterministic(
                &forester_keypair,
                1_000_000,
                Some(batched_state_merkle_tree_index),
            )
            .await;
        e2e_env
            .transfer_sol_deterministic(
                &forester_keypair,
                &Pubkey::new_unique(),
                Some(batched_state_merkle_tree_index),
            )
            .await
            .unwrap();
    }
    let (state_merkle_tree_bundle, _, _) = (
        e2e_env.indexer.state_merkle_trees[batched_state_merkle_tree_index].clone(),
        e2e_env.indexer.address_merkle_trees[batched_state_merkle_tree_index].clone(),
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

    let service_handle = tokio::spawn(run_pipeline::<LightClient, TestIndexer>(
        Arc::from(config.clone()),
        None,
        None,
        Arc::new(Mutex::new(e2e_env.indexer)),
        shutdown_receiver,
        work_report_sender,
    ));

    let timeout_duration = Duration::from_secs(60 * 10);
    match timeout(timeout_duration, work_report_receiver.recv()).await {
        Ok(Some(report)) => {
            println!("Received work report: {:?}", report);
            println!(
                "Work report debug:
                reported_items: {}
                batch_size: {}
                complete_batches: {}",
                report.processed_items,
                tree_params.input_queue_batch_size,
                report.processed_items / tree_params.input_queue_batch_size as usize,
            );
            assert!(report.processed_items > 0, "No items were processed");

            let zkp_batch_size = tree_params.input_queue_zkp_batch_size;
            assert_eq!(
                report.processed_items % zkp_batch_size as usize,
                0,
                "Processed items {} should be a multiple of batch size {}",
                report.processed_items,
                zkp_batch_size
            );
        }
        Ok(None) => panic!("Work report channel closed unexpectedly"),
        Err(_) => panic!("Test timed out after {:?}", timeout_duration),
    }

    let rpc = pool.get_connection().await.unwrap();
    let mut merkle_tree_account = rpc
        .get_account(batched_state_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();

    let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &batched_state_merkle_tree_pubkey.into(),
    )
    .unwrap();

    println!("Merkle tree metadata: {:?}", merkle_tree.get_metadata());
    assert!(
        merkle_tree.get_metadata().queue_batches.pending_batch_index > 0,
        "No batches were processed"
    );

    {
        let rpc = pool.get_connection().await.unwrap();

        let mut merkle_tree_account = rpc
            .get_account(batched_state_merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap();

        let merkle_tree = BatchedMerkleTreeAccount::state_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &batched_state_merkle_tree_pubkey.into(),
        )
        .unwrap();

        let final_metadata = merkle_tree.get_metadata();

        let mut output_queue_account = rpc
            .get_account(nullifier_queue_pubkey)
            .await
            .unwrap()
            .unwrap();

        let output_queue =
            BatchedQueueAccount::output_from_bytes(output_queue_account.data.as_mut_slice())
                .unwrap();

        let batch_size = merkle_tree.get_metadata().queue_batches.batch_size;
        let zkp_batch_size = merkle_tree.get_metadata().queue_batches.zkp_batch_size;
        let num_zkp_batches = batch_size / zkp_batch_size;

        let mut completed_items = 0;
        for batch_idx in 0..output_queue.batch_metadata.batches.len() {
            let batch = output_queue.batch_metadata.batches.get(batch_idx).unwrap();
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
            final_metadata.queue_batches,
            output_queue.get_metadata().batch_metadata
        );

        assert_eq!(
            final_metadata.next_index,
            initial_next_index + completed_items,
            "Merkle tree next_index did not advance by expected amount",
        );

        assert_eq!(
            merkle_tree.get_metadata().queue_batches.pending_batch_index,
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

        assert_eq!(final_metadata.sequence_number, expected_sequence_number);

        assert_eq!(
            merkle_tree.root_history.last_index(),
            expected_sequence_number as usize
        );

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
