use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use crate::constants::SERVER_URL;

pub fn request_airdrop(payer_pubkey: &Pubkey) {
    let client = RpcClient::new(SERVER_URL);
    let commitment_config = CommitmentConfig::finalized();
    let mut balance = client
        .get_balance_with_commitment(payer_pubkey, commitment_config)
        .unwrap()
        .value;
    println!("Old balance: {}", balance);
    while balance < 1000000000 {
        let latest_blockhash = client.get_latest_blockhash().unwrap();
        client
            .request_airdrop_with_blockhash(payer_pubkey, 1000000000, &latest_blockhash)
            .unwrap();
        balance = client
            .get_balance_with_commitment(payer_pubkey, commitment_config)
            .unwrap()
            .value;
        println!("Waiting for airdrop...");
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
    println!("New balance: {}", balance);
}