use account_compression::StateMerkleTreeAccount;
use anchor_lang::AccountDeserialize;
use forester::constants::{INDEXER_URL, SERVER_URL};
use forester::indexer::PhotonIndexer;
use forester::nullifier::{get_nullifier_queue, nullify, subscribe_nullify, Config};
use forester::utils::u8_arr_to_hex_string;
use log::{info, warn};
use solana_client::rpc_client::RpcClient;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn queue_info_test() {
    let config = Config::test();
    let client = RpcClient::new(config.server_url);
    let queue = get_nullifier_queue(&config.nullifier_queue_pubkey, &client).unwrap();
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
    let mut data_ref = data;
    let merkle_tree_account: StateMerkleTreeAccount =
        StateMerkleTreeAccount::try_deserialize(&mut data_ref).unwrap();
    let merkle_tree = merkle_tree_account.copy_merkle_tree().unwrap();

    let root = merkle_tree.root();
    info!("Merkle tree root: {:?}", u8_arr_to_hex_string(&root));
    info!(
        "Merkle tree rightmost leaf: {:?}",
        u8_arr_to_hex_string(&merkle_tree.rightmost_leaf)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn test_nullify_leaves() {
    let indexer = PhotonIndexer::new(INDEXER_URL.to_string());
    let config = Config::test();
    let client = RpcClient::new(SERVER_URL);
    client
        .request_airdrop(&config.payer_keypair.pubkey(), LAMPORTS_PER_SOL * 1000)
        .unwrap();

    let time = std::time::Instant::now();
    match nullify(indexer, &config).await {
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
    let config = Config::test();
    let client = RpcClient::new(SERVER_URL);
    client
        .request_airdrop(&config.payer_keypair.pubkey(), LAMPORTS_PER_SOL * 1000)
        .unwrap();
    subscribe_nullify(&config).await;
}
