use std::{sync::Arc, time::Duration};

use forester::run_pipeline;
use forester_utils::registry::{register_test_forester, update_test_forester};
use light_batched_merkle_tree::{
    batch::BatchState, initialize_address_tree::InitAddressTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount,
};
use light_client::{
    indexer::AddressMerkleTreeAccounts,
    rpc::{solana_rpc::SolanaRpcUrl, RpcConnection, SolanaRpcConnection},
    rpc_pool::SolanaRpcPool,
};
use light_program_test::{indexer::TestIndexer, test_env::EnvAccounts};
use light_prover_client::gnark::helpers::{LightValidatorConfig, ProverConfig, ProverMode};
use light_test_utils::{
    create_address_test_program_sdk::perform_create_pda_with_event_rnd, e2e_test_env::E2ETestEnv,
};
use serial_test::serial;
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    time::{sleep, timeout},
};
use tracing::log::info;

use crate::test_utils::{forester_config, general_action_config, init, keypair_action_config};

mod test_utils;

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
#[serial]
async fn test_address_batched() {
    init(Some(LightValidatorConfig {
        enable_indexer: false,
        wait_time: 60,
        prover_config: Some(ProverConfig {
            run_mode: Some(ProverMode::ForesterTest),
            circuits: vec![],
        }),
        sbf_programs: vec![(
            "FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy".to_string(),
            "../target/deploy/create_address_test_program.so".to_string(),
        )],
        limit_ledger_size: None,
    }))
    .await;

    let tree_params = InitAddressTreeAccountsInstructionData::test_default();

    let forester_keypair = Keypair::new();
    let mut env_accounts = EnvAccounts::get_local_test_validator_accounts();
    env_accounts.forester = forester_keypair.insecure_clone();

    let mut config = forester_config();
    config.transaction_config.batch_ixs_per_tx = 1;
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
        &env_accounts.governance_authority.pubkey(),
        LAMPORTS_PER_SOL * 100_000,
    )
    .await
    .unwrap();

    register_test_forester(
        &mut rpc,
        &env_accounts.governance_authority,
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

    let config = Arc::new(config);

    let indexer: TestIndexer<SolanaRpcConnection> =
        TestIndexer::init_from_env(&config.payer_keypair, &env_accounts, None).await;

    let mut env = E2ETestEnv::<SolanaRpcConnection, TestIndexer<SolanaRpcConnection>>::new(
        rpc,
        indexer,
        &env_accounts,
        keypair_action_config(),
        general_action_config(),
        0,
        Some(0),
    )
    .await;

    let (_address_merkle_tree_index, address_merkle_tree_pubkey) = env
        .indexer
        .address_merkle_trees
        .iter()
        .enumerate()
        .find(|(_index, tree)| tree.accounts.merkle_tree == tree.accounts.queue)
        .map(|(index, tree)| (index, tree.accounts.merkle_tree))
        .unwrap();

    let address_trees: Vec<AddressMerkleTreeAccounts> = env
        .indexer
        .address_merkle_trees
        .iter()
        .map(|x| x.accounts)
        .collect();

    println!("New address trees: {:?}", address_trees);
    for tree in address_trees {
        let is_v2 = tree.merkle_tree == tree.queue;
        println!("Tree {:?} is_v2: {}", tree, is_v2);
    }
    println!(
        "env.batch_address_merkle_tree , {}",
        env_accounts.batch_address_merkle_tree
    );
    let mut merkle_tree_account = env
        .rpc
        .get_account(address_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();
    println!("address_merkle_tree_pubkey {}", address_merkle_tree_pubkey);
    println!("merkle_tree_account {}", merkle_tree_account.owner);
    println!(
        "merkle_tree_account.data {:?}",
        merkle_tree_account.data[..100].to_vec()
    );
    let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
        &mut merkle_tree_account.data,
        &address_merkle_tree_pubkey.into(),
    )
    .unwrap();

    for i in 0..merkle_tree.queue_batches.batch_size {
        println!("===================== tx {} =====================", i);

        perform_create_pda_with_event_rnd(
            &mut env.indexer,
            &mut env.rpc,
            &env_accounts,
            &env.payer,
        )
        .await
        .unwrap();

        sleep(Duration::from_millis(100)).await;
    }

    let zkp_batches = tree_params.input_queue_batch_size / tree_params.input_queue_zkp_batch_size;

    println!("zkp_batches: {}", zkp_batches);

    let (initial_next_index, initial_sequence_number, pre_root) = {
        let mut rpc = pool.get_connection().await.unwrap();
        let mut merkle_tree_account = rpc
            .get_account(address_merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap();

        let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &address_merkle_tree_pubkey.into(),
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

    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (work_report_sender, mut work_report_receiver) = mpsc::channel(100);

    let service_handle = tokio::spawn(run_pipeline(
        config.clone(),
        Arc::new(Mutex::new(env.indexer)),
        shutdown_receiver,
        work_report_sender,
    ));

    let timeout_duration = Duration::from_secs(60 * 10);
    match timeout(timeout_duration, work_report_receiver.recv()).await {
        Ok(Some(report)) => {
            info!("Received work report: {:?}", report);
            assert!(report.processed_items > 0, "No items were processed");
        }
        Ok(None) => panic!("Work report channel closed unexpectedly"),
        Err(_) => panic!("Test timed out after {:?}", timeout_duration),
    }

    let mut rpc = pool.get_connection().await.unwrap();
    let mut merkle_tree_account = rpc
        .get_account(address_merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();

    let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
        merkle_tree_account.data.as_mut_slice(),
        &address_merkle_tree_pubkey.into(),
    )
    .unwrap();

    assert!(
        merkle_tree.get_metadata().queue_batches.pending_batch_index > 0,
        "No batches were processed"
    );

    {
        let mut rpc = pool.get_connection().await.unwrap();

        let mut merkle_tree_account = rpc
            .get_account(address_merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap();

        let merkle_tree = BatchedMerkleTreeAccount::address_from_bytes(
            merkle_tree_account.data.as_mut_slice(),
            &address_merkle_tree_pubkey.into(),
        )
        .unwrap();

        let final_metadata = merkle_tree.get_metadata();

        let batch_size = merkle_tree.get_metadata().queue_batches.batch_size;
        let zkp_batch_size = merkle_tree.get_metadata().queue_batches.zkp_batch_size;
        let num_zkp_batches = batch_size / zkp_batch_size;

        let mut completed_items = 0;
        for batch_idx in 0..merkle_tree.queue_batches.batches.len() {
            let batch = merkle_tree.queue_batches.batches.get(batch_idx).unwrap();
            if batch.get_state() == BatchState::Inserted {
                completed_items += batch_size;
            }
        }

        assert_eq!(
            final_metadata.next_index,
            initial_next_index + completed_items,
            "Merkle tree next_index did not advance by expected amount",
        );

        assert_eq!(
            merkle_tree.get_metadata().queue_batches.pending_batch_index,
            1
        );

        const UPDATES_PER_BATCH: u64 = 1;

        let expected_sequence_number =
            initial_sequence_number + (num_zkp_batches * UPDATES_PER_BATCH);
        let expected_root_history_len = expected_sequence_number as usize;

        assert_eq!(final_metadata.sequence_number, expected_sequence_number);

        assert_eq!(
            merkle_tree.root_history.last_index(),
            expected_root_history_len
        );

        assert_ne!(
            pre_root,
            merkle_tree.get_root().unwrap(),
            "Root should have changed"
        );
        assert!(
            merkle_tree.root_history.len() > 1,
            "Root history should contain multiple roots"
        );
    }

    shutdown_sender
        .send(())
        .expect("Failed to send shutdown signal");
    service_handle.await.unwrap().unwrap();
}
