use std::{sync::Arc, time::Duration};

use forester::run_pipeline;
use forester_utils::registry::{register_test_forester, update_test_forester};
use light_batched_merkle_tree::{
    batch::BatchState, initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount, queue::BatchedQueueAccount,
};
use light_client::{
    indexer::{photon_indexer::PhotonIndexer, Indexer},
    rpc::{solana_rpc::SolanaRpcUrl, RpcConnection, SolanaRpcConnection},
    rpc_pool::SolanaRpcPool,
};
use light_program_test::{indexer::TestIndexer, test_env::EnvAccounts};
use light_prover_client::gnark::helpers::LightValidatorConfig;
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
async fn test_state_indexer_batched() {
    let tree_params = InitStateTreeAccountsInstructionData::test_default();

    init(Some(LightValidatorConfig {
        enable_indexer: true,
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
        None,
        None,
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

    let photon_indexer = {
        let rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
        PhotonIndexer::new("http://127.0.0.1:8784".to_string(), None, rpc)
    };

    let mut e2e_env: E2ETestEnv<SolanaRpcConnection, TestIndexer<SolanaRpcConnection>>;
    e2e_env = init_program_test_env(rpc, &env, false).await;

    for tree in e2e_env.indexer.state_merkle_trees.iter() {
        println!("====================");
        println!("state merkle tree pub key: {}", tree.accounts.merkle_tree);
        println!("output queue pub key: {}", tree.accounts.nullifier_queue);
        println!("version: {}", tree.version);
    }

    let (batched_state_merkle_tree_index, batched_state_merkle_tree_pubkey, nullifier_queue_pubkey) =
        e2e_env
            .indexer
            .state_merkle_trees
            .iter()
            .enumerate()
            .find(|(_, tree)| tree.version == 2)
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
    let merkle_tree =
        BatchedMerkleTreeAccount::state_from_bytes(&mut merkle_tree_account.data).unwrap();

    let (initial_next_index, initial_sequence_number, pre_root) = {
        let mut rpc = pool.get_connection().await.unwrap();
        let mut merkle_tree_account = rpc
            .get_account(batched_state_merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap();

        let merkle_tree =
            BatchedMerkleTreeAccount::state_from_bytes(merkle_tree_account.data.as_mut_slice())
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

    println!(
        "get_compressed_accounts_by_owner({}) initial",
        &forester_keypair.pubkey()
    );
    let compressed_balance_photon = photon_indexer
        .get_compressed_accounts_by_owner(&forester_keypair.pubkey())
        .await
        .unwrap();
    let compressed_balance_test_indexer = e2e_env
        .indexer
        .get_compressed_accounts_by_owner(&forester_keypair.pubkey())
        .await
        .unwrap();
    for (photon_account, test_indexer_account) in compressed_balance_photon
        .iter()
        .zip(compressed_balance_test_indexer.iter())
    {
        assert_eq!(photon_account, test_indexer_account);
    }

    for i in 0..merkle_tree.get_metadata().queue_metadata.batch_size {
        println!("\ntx {}", i);

        e2e_env
            .compress_sol_deterministic(
                &forester_keypair,
                1_000_000,
                Some(batched_state_merkle_tree_index),
            )
            .await;

        println!(
            "get_compressed_accounts_by_owner({}) after compress_sol_deterministic",
            &forester_keypair.pubkey()
        );
        let compressed_balance_photon = photon_indexer
            .get_compressed_accounts_by_owner(&forester_keypair.pubkey())
            .await
            .unwrap();
        let compressed_balance_test_indexer = e2e_env
            .indexer
            .get_compressed_accounts_by_owner(&forester_keypair.pubkey())
            .await
            .unwrap();
        for (photon_account, test_indexer_account) in compressed_balance_photon
            .iter()
            .zip(compressed_balance_test_indexer.iter())
        {
            assert_eq!(photon_account, test_indexer_account);
        }

        let to_pubkey = Pubkey::new_unique();
        e2e_env
            .transfer_sol_deterministic(
                &forester_keypair,
                &to_pubkey,
                Some(batched_state_merkle_tree_index),
            )
            .await
            .unwrap();

        println!(
            "get_compressed_accounts_by_owner({}) after transfer_sol_deterministic",
            to_pubkey
        );
        let compressed_balance_photon = photon_indexer
            .get_compressed_accounts_by_owner(&to_pubkey)
            .await
            .unwrap();
        let compressed_balance_test_indexer = e2e_env
            .indexer
            .get_compressed_accounts_by_owner(&to_pubkey)
            .await
            .unwrap();
        for (photon_account, test_indexer_account) in compressed_balance_photon
            .iter()
            .zip(compressed_balance_test_indexer.iter())
        {
            assert_eq!(photon_account, test_indexer_account);
        }
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

    drop(e2e_env.indexer);

    let service_handle = tokio::spawn(run_pipeline(
        Arc::from(config.clone()),
        None,
        None,
        Arc::new(Mutex::new(photon_indexer)),
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
        .get_account(batched_state_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();

    let merkle_tree =
        BatchedMerkleTreeAccount::state_from_bytes(merkle_tree_account.data.as_mut_slice())
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
            .get_account(batched_state_merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap();

        let merkle_tree =
            BatchedMerkleTreeAccount::state_from_bytes(merkle_tree_account.data.as_mut_slice())
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

        let batch_size = merkle_tree.get_metadata().queue_metadata.batch_size;
        let zkp_batch_size = merkle_tree.get_metadata().queue_metadata.zkp_batch_size;
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
