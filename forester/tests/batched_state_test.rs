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
        wait_time: 60,
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

    let pre_root = {
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
        merkle_tree.get_root().unwrap()
    };

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
            assert!(report.processed_items > 0, "No items were processed");
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

    let post_root = {
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
        merkle_tree.get_root().unwrap()
    };

    assert_ne!(pre_root, post_root, "Roots are the same");

    shutdown_sender
        .send(())
        .expect("Failed to send shutdown signal");
    service_handle.await.unwrap().unwrap();
}
