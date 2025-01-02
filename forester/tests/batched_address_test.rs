use std::{sync::Arc, time::Duration};

use forester::run_pipeline;
use forester_utils::registry::{register_test_forester, update_test_forester};
use light_batched_merkle_tree::{
    initialize_state_tree::InitStateTreeAccountsInstructionData,
    merkle_tree::BatchedMerkleTreeAccount,
};
use light_client::{
    rpc::{solana_rpc::SolanaRpcUrl, RpcConnection, SolanaRpcConnection},
    rpc_pool::SolanaRpcPool,
};
use light_program_test::test_env::EnvAccounts;
use light_prover_client::gnark::helpers::{spawn_prover, LightValidatorConfig, ProverConfig, ProverMode};
use light_test_utils::{
    e2e_test_env::{init_program_test_env, E2ETestEnv},
    indexer::TestIndexer,
};
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer,
};
use tokio::{
    sync::{mpsc, oneshot, Mutex},
    time::timeout,
};
use tokio::time::sleep;
use tracing::log::info;
use forester_utils::indexer::AddressMerkleTreeAccounts;
use light_batched_merkle_tree::initialize_address_tree::InitAddressTreeAccountsInstructionData;
use light_client::rpc::assert_rpc_error;
use light_program_test::test_batch_forester::create_batch_address_merkle_tree;
use light_registry::errors::RegistryError;
use light_test_utils::create_address_test_program_sdk::perform_create_pda_with_event_rnd;
use crate::test_utils::{forester_config, general_action_config, init, keypair_action_config};

mod test_utils;

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
async fn test_address_batched() {
    init(Some(LightValidatorConfig {
        enable_indexer: false,
        wait_time: 15,
        prover_config: Some(ProverConfig {
            run_mode: Some(ProverMode::ForesterTest),
            circuits: vec![],
        }),
        sbf_programs: vec![("FNt7byTHev1k5x2cXZLBr8TdWiC3zoP5vcnZR4P682Uy".to_string(), "../target/deploy/create_address_test_program.so".to_string())],
    }))
    .await;

    let tree_params = InitAddressTreeAccountsInstructionData::test_default();

    let forester_keypair = Keypair::new();
    let mut env_accounts = EnvAccounts::get_local_test_validator_accounts();
    env_accounts.forester = forester_keypair.insecure_clone();

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

    let address_trees: Vec<AddressMerkleTreeAccounts> = env
        .indexer
        .address_merkle_trees
        .iter()
        .map(|x| x.accounts)
        .collect();

    println!("Address trees: {:?}", address_trees);
    for tree in address_trees {
        let is_v2 = tree.merkle_tree == tree.queue;
        println!("Tree {:?} is_v2: {}", tree, is_v2);
    }

    println!("Removing trees...");
    env.indexer.address_merkle_trees.clear();

    println!("Creating new address batch tree...");
    {
        let new_merkle_tree = Keypair::new();
        // let test_tree_params = InitAddressTreeAccountsInstructionData::default();
        // // test_tree_params.network_fee = Some(1);
        // let result =
        //     create_batch_address_merkle_tree(&mut env.rpc, &env.payer, &new_merkle_tree, test_tree_params)
        //         .await;
        env.indexer.add_address_merkle_tree(&mut env.rpc, &new_merkle_tree, &new_merkle_tree, None, 2).await;
        env_accounts.batch_address_merkle_tree = new_merkle_tree.pubkey();
    }

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



    for i in 0..50 {
        println!("===================== tx {} =====================", i);
        // env.create_address(None, Some(0)).await;

        perform_create_pda_with_event_rnd(&mut env.indexer, &mut env.rpc, &env_accounts, &env.payer)
            .await
            .unwrap();

        sleep(Duration::from_millis(100)).await;

    }

    let merkle_tree_pubkey = env.indexer.address_merkle_trees[0].accounts.merkle_tree;

    let zkp_batches = tree_params.input_queue_batch_size / tree_params.input_queue_zkp_batch_size;

    println!("zkp_batches: {}", zkp_batches);

    let pre_root = {
        let mut rpc = pool.get_connection().await.unwrap();
        let mut merkle_tree_account = rpc
            .get_account(merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap();

        let merkle_tree = BatchedMerkleTreeAccount::address_tree_from_bytes_mut(
            merkle_tree_account.data.as_mut_slice(),
        )
        .unwrap();
        merkle_tree.get_root().unwrap()
    };

    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (work_report_sender, mut work_report_receiver) = mpsc::channel(100);

    let service_handle = tokio::spawn(run_pipeline(
        Arc::from(config.clone()),
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
        .get_account(merkle_tree_pubkey)
        .await
        .unwrap()
        .unwrap();

    let merkle_tree = BatchedMerkleTreeAccount::address_tree_from_bytes_mut(
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

    let post_root = {
        let mut rpc = pool.get_connection().await.unwrap();
        let mut merkle_tree_account = rpc
            .get_account(merkle_tree_pubkey)
            .await
            .unwrap()
            .unwrap();

        let merkle_tree = BatchedMerkleTreeAccount::address_tree_from_bytes_mut(
            merkle_tree_account.data.as_mut_slice(),
        )
        .unwrap();
        merkle_tree.get_root().unwrap()
    };

    assert_ne!(pre_root, post_root, "Roots are the same");

    shutdown_sender
        .send(())
        .expect("Failed to send shutdown signal");
    service_handle.await.unwrap().unwrap();
}
