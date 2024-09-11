use forester::photon_indexer::PhotonIndexer;
use forester::queue_helpers::fetch_queue_item_data;
use forester::utils::LightValidatorConfig;
use forester_utils::rpc::solana_rpc::SolanaRpcUrl;
use forester_utils::rpc::{RpcConnection, SolanaRpcConnection};
use light_test_utils::e2e_test_env::E2ETestEnv;
use light_test_utils::indexer::TestIndexer;
use light_test_utils::test_env::EnvAccounts;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::time::Duration;
use tokio::time::sleep;
mod test_utils;
use forester::run_pipeline;
use forester_utils::indexer::Indexer;
use std::sync::Arc;
use test_utils::*;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::info;

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
async fn test_multiple_state_trees_with_photon() {
    init(Some(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        enable_forester: false,
        wait_time: 10,
        ..LightValidatorConfig::default()
    }))
    .await;
    let photon_indexer = create_local_photon_indexer();
    let env_accounts = EnvAccounts::get_local_test_validator_accounts();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), 10_000_000_000_000)
        .await
        .unwrap();

    let indexer: TestIndexer<SolanaRpcConnection> =
        TestIndexer::init_from_env(&rpc.get_payer(), &env_accounts, false, false).await;

    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), 10_000_000_000_000)
        .await
        .unwrap();
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

    for i in 0..10 {
        let new_mt_pubkey = env.create_state_tree(Some(95)).await;
        let new_keypair = Keypair::new();
        let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
        rpc.airdrop_lamports(&new_keypair.pubkey(), LAMPORTS_PER_SOL * 1)
            .await
            .unwrap();
        env.compress_sol_deterministic(&new_keypair, 1_000_000, Some(i + 1))
            .await;
        sleep(Duration::from_secs(2)).await;
        use forester_utils::indexer::Indexer;
        let compressed_accounts = photon_indexer
            .get_compressed_accounts_by_owner(&new_keypair.pubkey())
            .await;
        assert_eq!(compressed_accounts.len(), 1);
        assert_eq!(
            compressed_accounts[0].compressed_account.lamports,
            1_000_000
        );
        assert_eq!(
            new_mt_pubkey,
            compressed_accounts[0].merkle_context.merkle_tree_pubkey
        );
        info!("user {:?}", new_keypair.pubkey());
        info!(
            "state Merkle tree len {:?}",
            compressed_accounts[0].compressed_account
        );
        info!("new_mt_pubkey: {:?}", new_mt_pubkey);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
async fn test_multiple_address_trees_with_photon() {
    init(Some(LightValidatorConfig {
        enable_indexer: true,
        enable_prover: true,
        enable_forester: false,
        wait_time: 20,
        ..LightValidatorConfig::default()
    }))
    .await;
    let photon_indexer = create_local_photon_indexer();
    let forester_photon_indexer = create_local_photon_indexer();
    let env_accounts = EnvAccounts::get_local_test_validator_accounts();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), 10_000_000_000_000)
        .await
        .unwrap();
    let (shutdown_sender, shutdown_receiver) = oneshot::channel();
    let (work_report_sender, _work_report_receiver) = mpsc::channel(100);

    let mut config = forester_config();
    config.payer_keypair = env_accounts.forester.insecure_clone();

    let config = Arc::new(config);
    // Run the forester as pipeline
    let service_handle = tokio::spawn(run_pipeline(
        config.clone(),
        Arc::new(Mutex::new(forester_photon_indexer)),
        shutdown_receiver,
        work_report_sender,
    ));

    let indexer: TestIndexer<SolanaRpcConnection> =
        TestIndexer::init_from_env(&rpc.get_payer(), &env_accounts, false, false).await;

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

    for i in 0..10 {
        let address_tree_accounts = env.create_address_tree(Some(95)).await;
        tokio::time::sleep(Duration::from_secs(2)).await;

        info!("address_tree_accounts {:?}", address_tree_accounts);
        info!(
            "address_tree_accounts.merkle_tree {:?}",
            address_tree_accounts.merkle_tree.to_bytes()
        );
        info!(
            "address_tree_accounts.queue {:?}",
            address_tree_accounts.queue
        );
        let init_seed = Pubkey::new_unique();
        let init_address_proof = photon_indexer
            .get_multiple_new_address_proofs(
                address_tree_accounts.merkle_tree.to_bytes(),
                vec![init_seed.to_bytes()],
            )
            .await
            .unwrap();
        let seed = Pubkey::new_unique();
        env.create_address(Some(vec![seed]), Some(i + 1)).await;
        assert!(address_queue_len_is_equal_to(&mut env.rpc, address_tree_accounts.queue, 1).await);
        while !address_queue_len_is_equal_to(&mut env.rpc, address_tree_accounts.queue, 0).await {
            sleep(Duration::from_secs(1)).await;
            info!("sleeping until address queue is empty");
        }
        let address_proof = photon_indexer
            .get_multiple_new_address_proofs(
                address_tree_accounts.merkle_tree.to_bytes(),
                vec![init_seed.to_bytes()],
            )
            .await
            .unwrap();
        assert_ne!(init_address_proof, address_proof);
    }
    shutdown_sender
        .send(())
        .expect("Failed to send shutdown signal");
    service_handle.await.unwrap().unwrap();
}

// TODO: make static method of PhotonIndexer
pub fn create_local_photon_indexer() -> PhotonIndexer<SolanaRpcConnection> {
    let rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    PhotonIndexer::new(String::from("http://127.0.0.1:8784"), None, rpc)
}
pub async fn address_queue_len_is_equal_to(
    rpc: &mut SolanaRpcConnection,
    queue: Pubkey,
    expected_len: u64,
) -> bool {
    let queue_length = fetch_queue_item_data(&mut *rpc, &queue)
        .await
        .unwrap()
        .len() as u64;
    queue_length == expected_len
}
