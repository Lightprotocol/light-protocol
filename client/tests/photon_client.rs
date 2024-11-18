use std::time::Duration;

use light_client::{
    photon_rpc::{AddressWithTree, PhotonClient},
    rpc::SolanaRpcConnection,
};
use light_prover_client::gnark::helpers::{spawn_validator, LightValidatorConfig, ProofType, ProverConfig, ProverMode};
use light_test_utils::test_env::EnvAccounts;
use light_system_program::sdk::{
    compressed_account::CompressedAccount,
    invoke::create_invoke_instruction,
};
use solana_sdk::{
    signer::Signer,
    transaction::Transaction,
    native_token::LAMPORTS_PER_SOL,
};
use light_test_utils::RpcConnection;


#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_all_endpoints() {
    println!("Test starting...");
    
    // Set explicit memory limits
    #[cfg(target_os = "macos")]
    std::env::set_var("RUST_MIN_STACK", "4194304"); // 4MB stack
    
    println!("Spawning validator...");
    let config = LightValidatorConfig {
        enable_indexer: true,
        prover_config: Some(ProverConfig {
            run_mode: Some(ProverMode::Rpc),
            circuits: vec![],
        }),
        wait_time: 15,
    };
    
    // Add explicit error handling
    spawn_validator(config).await;
    
    println!("Validator spawn completed");
    tokio::time::sleep(Duration::from_secs(5)).await;

    let env_accounts = EnvAccounts::get_local_test_validator_accounts();
    let rpc = SolanaRpcConnection::new("http://127.0.0.1:8899".to_string(), None);
    println!("RPC");
    let client = PhotonClient::new("http://127.0.0.1:8784".to_string());
    println!("CLIENT");
    let payer = rpc.get_payer();

    // Create and send compress transaction
    let output_account = CompressedAccount {
        lamports: LAMPORTS_PER_SOL / 2,
        owner: payer.pubkey(),
        data: None,
        address: None,
    };

    let ix = create_invoke_instruction(
        &payer.pubkey(),
        &payer.pubkey(),
        &[],
        &[output_account],
        &[],
        &[env_accounts.merkle_tree_pubkey],
        &[],
        &[],
        None,
        None,
        false,
        None,
        true,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.client.get_latest_blockhash().unwrap(),
    );
    rpc.client.send_and_confirm_transaction(&tx).unwrap();
    
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Test endpoints
    let pubkey = payer.pubkey();
    let hashes = client.get_rpc_compressed_accounts_by_owner(&pubkey).await.unwrap();
    assert!(!hashes.is_empty());

    let first_hash = hashes[0];
    let new_addresses = vec![AddressWithTree {
        address: [0u8; 32],
        tree: env_accounts.address_merkle_tree_pubkey,
    }];

    assert!(client.get_validity_proof(hashes.clone(), new_addresses).await.is_ok());
    assert!(client.get_compressed_account(None, Some(first_hash)).await.is_ok());
    assert!(client.get_compressed_account_balance(None, Some(first_hash)).await.is_ok());
    assert!(client.get_multiple_compressed_accounts(vec![], hashes.clone()).await.is_ok());
    assert!(client.get_compression_signatures_for_account(first_hash).await.is_ok());

    // Token endpoints
    assert!(client.get_compressed_token_accounts_by_owner(&pubkey, None).await.is_ok());
    assert!(client.get_compressed_token_account_balance(None, Some(first_hash)).await.is_ok());
    assert!(client.get_compressed_token_balances_by_owner(&pubkey, None).await.is_ok());
}
