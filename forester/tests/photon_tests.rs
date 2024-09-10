use forester::photon_indexer::PhotonIndexer;
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
use forester_utils::indexer::Indexer;
use test_utils::*;
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

#[ignore = "Test fails possibly because of photon"]
#[tokio::test(flavor = "multi_thread", worker_threads = 32)]
async fn test_multiple_address_trees_with_photon() {
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
        let address_merkle_tree = env.create_address_tree(Some(95)).await;

        let init_seed = Pubkey::new_unique();
        let init_address_proof = photon_indexer
            .get_multiple_new_address_proofs(
                address_merkle_tree.to_bytes(),
                vec![init_seed.to_bytes()],
            )
            .await
            .unwrap();
        let seed = Pubkey::new_unique();
        env.create_address(Some(vec![seed]), Some(i + 1)).await;
        sleep(Duration::from_secs(2)).await;
        let address_proof = photon_indexer
            .get_multiple_new_address_proofs(
                address_merkle_tree.to_bytes(),
                vec![init_seed.to_bytes()],
            )
            .await
            .unwrap();
        assert_ne!(init_address_proof, address_proof);
        info!("address proof {:?}", address_proof);
    }
}
// TODO: make static method of PhotonIndexer
pub fn create_local_photon_indexer() -> PhotonIndexer<SolanaRpcConnection> {
    let rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    PhotonIndexer::new(String::from("http://127.0.0.1:8784"), None, rpc)
}
