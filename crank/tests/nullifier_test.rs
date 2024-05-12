use crank::constants::SERVER_URL;
use crank::nullifier::{get_nullifier_queue, nullify, subscribe_nullify};
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use std::process::Command;
use std::sync::Arc;
use sysinfo::{Signal, System};
use account_compression::StateMerkleTreeAccount;
use anchor_lang::AccountDeserialize;
use crank::utils::request_airdrop;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn queue_info_test() {
    let nullifier_queue =
        "5T2Fg9GVnZjGJetLnt2HF1CpYMM9fAzxodvmqJzh8dgjs96hqkwtcXkYrg7wT2ZCGj6syhAYtg5EEpeDBTQDJGY5";
    let nullifier_queue_keypair = Keypair::from_base58_string(nullifier_queue);
    let nullifier_queue_pubkey = nullifier_queue_keypair.pubkey();

    let client = RpcClient::new(SERVER_URL);
    let queue = get_nullifier_queue(&nullifier_queue_pubkey, &client).unwrap();
    println!("Nullifier queue length: {}", queue.len());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tree_info_test() {
    // restart_photon().await;
    let merkle_tree =
        "3wBL7d5qoWiYAV2bHMsmjKFr3u8SWa4Aps9mAcanfhRQMdFrtJtASwB5ZSvYeoAgD3SZsiYtnZVrrXpHKDpxkgZ2";
    let nullifier_queue =
        "5T2Fg9GVnZjGJetLnt2HF1CpYMM9fAzxodvmqJzh8dgjs96hqkwtcXkYrg7wT2ZCGj6syhAYtg5EEpeDBTQDJGY5";
    let payer = "LsYPAULcTDhjnECes7qhwAdeEUVYgbpX5ri5zijUceTQXCwkxP94zKdG4pmDQmicF7Zbj1AqB44t8qfGE8RuUk8";

    let nullifier_queue_pubkey = Keypair::from_base58_string(nullifier_queue);
    let nullifier_queue_pubkey = nullifier_queue_pubkey.pubkey();

    let merkle_tree_keypair = Keypair::from_base58_string(merkle_tree);
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();

    let payer_keypair = Keypair::from_base58_string(payer);
    let payer_pubkey = payer_keypair.pubkey();

    println!("Payer pubkey: {:?}", payer_pubkey);

    println!(
        "Nullify compressed accounts for nullifier queue: {} and merkle tree: {}",
        nullifier_queue_pubkey, merkle_tree_pubkey
    );

    let client = RpcClient::new(SERVER_URL);

    request_airdrop(&payer_pubkey);

    let data: &[u8] = &client.get_account_data(&merkle_tree_pubkey).unwrap();
    let mut data_ref = &data[..];
    let merkle_tree_account: StateMerkleTreeAccount =
        StateMerkleTreeAccount::try_deserialize(&mut data_ref).unwrap();
    let merkle_tree = merkle_tree_account.copy_merkle_tree().unwrap();

    // println!("Merkle tree: {:?}", merkle_tree);

    let root = merkle_tree.root().unwrap();
    println!("Merkle tree root: {:?}", u8_arr_to_hex_string(&root));
    println!("Merkle tree rightmost leaf: {:?}", u8_arr_to_hex_string(&merkle_tree.rightmost_leaf));
}

fn u8_arr_to_hex_string(arr: &[u8]) -> String {
    arr.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join("")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_nullify_leaves() {
    // restart_photon().await;
    let merkle_tree =
        "3wBL7d5qoWiYAV2bHMsmjKFr3u8SWa4Aps9mAcanfhRQMdFrtJtASwB5ZSvYeoAgD3SZsiYtnZVrrXpHKDpxkgZ2";
    let nullifier_queue =
        "5T2Fg9GVnZjGJetLnt2HF1CpYMM9fAzxodvmqJzh8dgjs96hqkwtcXkYrg7wT2ZCGj6syhAYtg5EEpeDBTQDJGY5";
    // let payer =
    //     "LsYPAULcTDhjnECes7qhwAdeEUVYgbpX5ri5zijUceTQXCwkxP94zKdG4pmDQmicF7Zbj1AqB44t8qfGE8RuUk8";

    let payer = [
        46, 239, 29, 58, 196, 181, 39, 77, 196, 54, 249, 108, 80, 144, 32, 168, 245, 161, 146, 92,
        180, 79, 231, 37, 50, 88, 220, 48, 9, 146, 249, 82, 130, 60, 106, 251, 24, 224, 192, 108,
        70, 59, 111, 251, 186, 50, 23, 103, 106, 233, 113, 148, 57, 190, 158, 111, 163, 28, 157,
        47, 201, 41, 249, 59,
    ];

    let nullifier_queue_pubkey = Keypair::from_base58_string(nullifier_queue);
    let nullifier_queue_pubkey = nullifier_queue_pubkey.pubkey();

    let merkle_tree_keypair = Keypair::from_base58_string(merkle_tree);
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();

    let payer_keypair = Keypair::from_bytes(&payer).unwrap();
    let payer_pubkey = payer_keypair.pubkey();

    println!("Payer pubkey: {:?}", payer_pubkey);

    println!(
        "Nullify compressed accounts for nullifier queue: {} and merkle tree: {}",
        nullifier_queue_pubkey, merkle_tree_pubkey
    );

    request_airdrop(&payer_pubkey);

    let payer_keypair = Arc::new(payer_keypair);

    let time = std::time::Instant::now();
    match nullify(
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        payer_keypair,
        SERVER_URL.to_string(),
    )
    .await
    {
        Ok(_) => {
            println!("Nullify completed");
            println!("Total time elapsed: {:?}", time.elapsed());
        }
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
}


#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_subscribe_nullify() {
    // restart_photon().await;
    let merkle_tree =
        "3wBL7d5qoWiYAV2bHMsmjKFr3u8SWa4Aps9mAcanfhRQMdFrtJtASwB5ZSvYeoAgD3SZsiYtnZVrrXpHKDpxkgZ2";
    let nullifier_queue =
        "5T2Fg9GVnZjGJetLnt2HF1CpYMM9fAzxodvmqJzh8dgjs96hqkwtcXkYrg7wT2ZCGj6syhAYtg5EEpeDBTQDJGY5";
    // let payer =
    //     "LsYPAULcTDhjnECes7qhwAdeEUVYgbpX5ri5zijUceTQXCwkxP94zKdG4pmDQmicF7Zbj1AqB44t8qfGE8RuUk8";

    let payer = [
        46, 239, 29, 58, 196, 181, 39, 77, 196, 54, 249, 108, 80, 144, 32, 168, 245, 161, 146, 92,
        180, 79, 231, 37, 50, 88, 220, 48, 9, 146, 249, 82, 130, 60, 106, 251, 24, 224, 192, 108,
        70, 59, 111, 251, 186, 50, 23, 103, 106, 233, 113, 148, 57, 190, 158, 111, 163, 28, 157,
        47, 201, 41, 249, 59,
    ];

    let nullifier_queue_pubkey = Keypair::from_base58_string(nullifier_queue);
    let nullifier_queue_pubkey = nullifier_queue_pubkey.pubkey();

    let merkle_tree_keypair = Keypair::from_base58_string(merkle_tree);
    let merkle_tree_pubkey = merkle_tree_keypair.pubkey();

    let payer_keypair = Keypair::from_bytes(&payer).unwrap();
    let payer_pubkey = payer_keypair.pubkey();

    println!("Payer pubkey: {:?}", payer_pubkey);

    println!(
        "Nullify compressed accounts for nullifier queue: {} and merkle tree: {}",
        nullifier_queue_pubkey, merkle_tree_pubkey
    );
    request_airdrop(&payer_pubkey);
    subscribe_nullify(
        &nullifier_queue_pubkey,
        &merkle_tree_pubkey,
        payer_keypair,
    ).await;
}

pub async fn restart_photon() {
    kill_photon();
    Command::new("photon")
        .spawn()
        .expect("Failed to start server process");
}

pub fn kill_photon() {
    let mut system = System::new_all();
    system.refresh_all();

    for process in system.processes().values() {
        if process.name() == "photon" {
            process.kill_with(Signal::Term);
        }
    }
}