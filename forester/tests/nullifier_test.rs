use std::mem;

use account_compression::StateMerkleTreeAccount;
use anchor_lang::AccountDeserialize;
use forester::constants::{INDEXER_URL, SERVER_URL};
use forester::indexer::PhotonIndexer;
use forester::nullifier::{get_nullifier_queue, nullify, subscribe_nullify, Config};
use forester::utils::u8_arr_to_hex_string;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};
use log::{info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

fn test_config() -> Config {
    let registry_keypair = Keypair::from_bytes(&REGISTRY_ID_TEST_KEYPAIR).unwrap();

    let env_accounts = get_test_env_accounts();
    Config {
        server_url: SERVER_URL.to_string(),
        nullifier_queue_pubkey: env_accounts.nullifier_queue_pubkey,
        state_merkle_tree_pubkey: env_accounts.merkle_tree_pubkey,
        address_merkle_tree_pubkey: env_accounts.address_merkle_tree_pubkey,
        address_merkle_tree_queue_pubkey: env_accounts.address_merkle_tree_queue_pubkey,
        registry_pubkey: registry_keypair.pubkey(),
        payer_keypair: env_accounts.governance_authority.insecure_clone(),
        concurrency_limit: 20,
        batch_size: 1000,
        max_retries: 5,
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn queue_info_test() {
    let config = test_config();
    let mut rpc = SolanaRpcConnection::new(None).await;
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

    let client = RpcClient::new(SERVER_URL);
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
    let mut indexer = PhotonIndexer::new(INDEXER_URL.to_string());
    let config = test_config();
    let mut rpc = SolanaRpcConnection::new(None).await;
    let _ = rpc
        .airdrop_lamports(&config.payer_keypair.pubkey(), LAMPORTS_PER_SOL * 1000)
        .await;

    let time = std::time::Instant::now();
    match nullify(&mut indexer, &mut rpc, &config).await {
        Ok(_) => {
            info!("Nullify completed");
            info!("Total time elapsed: {:?}", time.elapsed());
        }
        Err(e) => {
            warn!("Error: {:?}", e);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn test_subscribe_nullify() {
    let config = test_config();
    let mut rpc = SolanaRpcConnection::new(None).await;
    let _ = rpc
        .airdrop_lamports(&config.payer_keypair.pubkey(), LAMPORTS_PER_SOL * 1000)
        .await;
    subscribe_nullify(&config, &mut rpc).await;
}
