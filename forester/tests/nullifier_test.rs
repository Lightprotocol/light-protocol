use std::mem;
use std::sync::Arc;

use log::info;
use solana_client::rpc_client::RpcClient;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

use account_compression::StateMerkleTreeAccount;
use forester::external_services_config::ExternalServicesConfig;
use forester::nullifier::state::get_nullifier_queue;
use forester::utils::u8_arr_to_hex_string;
use forester::{nullify_state, subscribe_state, ForesterConfig};
use light_concurrent_merkle_tree::copy::ConcurrentMerkleTreeCopy;
use light_hasher::Poseidon;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};

fn test_config() -> ForesterConfig {
    let registry_keypair = Keypair::from_bytes(&REGISTRY_ID_TEST_KEYPAIR).unwrap();

    let env_accounts = get_test_env_accounts();
    ForesterConfig {
        external_services: ExternalServicesConfig {
            rpc_url: "http://localhost:8899".to_string(),
            ws_rpc_url: "ws://localhost:8900".to_string(),
            indexer_url: "http://localhost:8784".to_string(),
            prover_url: "http://localhost:3001".to_string(),
            derivation: "En9a97stB3Ek2n6Ey3NJwCUJnmTzLMMEA5C69upGDuQP".to_string(),
        },
        nullifier_queue_pubkey: env_accounts.nullifier_queue_pubkey,
        state_merkle_tree_pubkey: env_accounts.merkle_tree_pubkey,
        address_merkle_tree_pubkey: env_accounts.address_merkle_tree_pubkey,
        address_merkle_tree_queue_pubkey: env_accounts.address_merkle_tree_queue_pubkey,
        registry_pubkey: registry_keypair.pubkey(),
        payer_keypair: env_accounts.governance_authority.insecure_clone(),
        concurrency_limit: 20,
        batch_size: 1000,
        max_retries: 5,
        max_concurrent_batches: 5,
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn queue_info_test() {
    let config = test_config();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    let queue = get_nullifier_queue(&config.nullifier_queue_pubkey, &mut rpc)
        .await
        .unwrap();
    info!("Nullifier queue length: {}", queue.len());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn tree_info_test() {
    let merkle_tree =
        "3wBL7d5qoWiYAV2bHMsmjKFr3u8SWa4Aps9mAcanfhRQMdFrtJtASwB5ZSvYeoAgD3SZsiYtnZVrrXpHKDpxkgZ2";
    let nullifier_queue =
        "5T2Fg9GVnZjGJetLnt2HF1CpYMM9fAzxodvmqJzh8dgjs96hqkwtcXkYrg7wT2ZCGj6syhAYtg5EEpeDBTQDJGY5";
    let payer =
        "LsYPAULcTDhjnECes7qhwAdeEUVYgbpX5ri5zijUceTQXCwkxP94zKdG4pmDQmicF7Zbj1AqB44t8qfGE8RuUk8";

    let nullifier_queue_pubkey = Keypair::from_base58_string(nullifier_queue);
    let nullifier_queue_pubkey = nullifier_queue_pubkey.pubkey();

    let merkle_tree_keypair = Keypair::from_base58_string(merkle_tree);
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();

    let payer_keypair = Keypair::from_base58_string(payer);
    let payer_pubkey = payer_keypair.pubkey();

    info!("Payer pubkey: {:?}", payer_pubkey);
    info!(
        "Nullify compressed accounts for nullifier queue: {} and merkle tree: {}",
        nullifier_queue_pubkey, merkle_tree_pubkey
    );

    let client = RpcClient::new(test_config().external_services.rpc_url);
    client
        .request_airdrop(&payer_pubkey, LAMPORTS_PER_SOL * 1000)
        .unwrap();

    let data: &[u8] = &client.get_account_data(&merkle_tree_pubkey).unwrap();
    let merkle_tree = ConcurrentMerkleTreeCopy::<Poseidon, 26>::from_bytes_copy(
        &data[8 + mem::size_of::<StateMerkleTreeAccount>()..],
    )
    .unwrap();

    let root = merkle_tree.root();
    info!("Merkle tree root: {:?}", u8_arr_to_hex_string(&root));
    info!(
        "Merkle tree rightmost leaf: {:?}",
        u8_arr_to_hex_string(&merkle_tree.rightmost_leaf())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn test_nullify_leaves() {
    let config = Arc::new(test_config());
    let rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    let rpc = Arc::new(tokio::sync::Mutex::new(rpc));
    rpc.lock()
        .await
        .airdrop_lamports(&config.payer_keypair.pubkey(), LAMPORTS_PER_SOL * 1000)
        .await
        .unwrap();

    nullify_state(config).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn test_subscribe_nullify() {
    let config = test_config();
    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);
    rpc.airdrop_lamports(&config.payer_keypair.pubkey(), LAMPORTS_PER_SOL * 1000)
        .await
        .unwrap();
    subscribe_state(Arc::new(config)).await;
}
