use solana_client::pubsub_client::PubsubClient;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Keypair;
use account_compression::Pubkey;
use crate::constants::WS_SERVER_URL;
use crate::nullifier::nullify_compressed_accounts;

pub fn subscribe_nullify(
    nullifier_queue_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    payer_keypair: &Keypair,
    client: &RpcClient,
) {
    let (_account_subscription_client, account_subscription_receiver) =
        PubsubClient::account_subscribe(
            WS_SERVER_URL,
            nullifier_queue_pubkey,
            Some(RpcAccountInfoConfig {
                encoding: None,
                data_slice: None,
                commitment: Some(CommitmentConfig::confirmed()),
                min_context_slot: None,
            }),
        )
            .unwrap();

    loop {
        match account_subscription_receiver.recv() {
            Ok(_) => {
                nullify_compressed_accounts(nullifier_queue_pubkey, merkle_tree_pubkey, payer_keypair, client);
            }
            Err(e) => {
                println!("account subscription error: {:?}", e);
                break;
            }
        }
    }
}