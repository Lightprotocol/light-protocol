use std::sync::Arc;
use std::time::Duration;

use forester::compressible::{state::CompressibleAccountTracker, subscriber::AccountSubscriber};
use light_client::{
    local_test_validator::{spawn_validator, LightValidatorConfig},
    rpc::{LightClient, LightClientConfig, Rpc},
};
use light_compressed_token_sdk::instructions::create_compressed_mint;
use light_ctoken_types::state::TokenDataVersion;
use light_token_client::actions::{
    create_compressible_token_account, CreateCompressibleTokenAccountInputs,
};
use serial_test::serial;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use tokio::sync::oneshot;
use tokio::time::sleep;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[serial]
async fn test_account_subscriber_receives_updates() {
    // 1. Start local validator with indexer
    let config = LightValidatorConfig {
        enable_indexer: true,
        enable_prover: false,
        wait_time: 10,
        sbf_programs: vec![],
        limit_ledger_size: None,
        grpc_port: None,
    };
    spawn_validator(config).await;

    // 2. Initialize LightClient
    let mut rpc = LightClient::new(LightClientConfig::local())
        .await
        .expect("Failed to create LightClient");
    rpc.get_latest_active_state_trees()
        .await
        .expect("Failed to get state trees");

    let payer = rpc.get_payer().insecure_clone();
    let payer_pubkey = payer.pubkey();

    // Ensure sufficient balance
    rpc.airdrop_lamports(&payer_pubkey, 10_000_000_000)
        .await
        .expect("Failed to airdrop lamports");

    // 3. Create CompressibleAccountTracker and setup shutdown channel
    let tracker = Arc::new(CompressibleAccountTracker::new());

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    // 4. Create subscriber
    let ws_url = "ws://localhost:8900".to_string();
    let tracker_clone = tracker.clone();
    let mut subscriber = AccountSubscriber::new(ws_url, tracker_clone, shutdown_rx);

    // 5. Spawn subscriber in background
    let subscriber_handle = tokio::spawn(async move {
        subscriber.run().await.expect("Subscriber failed to run");
    });

    // Give subscriber time to connect
    sleep(Duration::from_secs(2)).await;
    let mint_seed = Keypair::new();
    // Derive the compressed mint address
    let address_tree = rpc.get_address_tree_v2().tree;
    let mint = Pubkey::from(create_compressed_mint::derive_compressed_mint_address(
        &mint_seed.pubkey(),
        &address_tree,
    ));

    // 7. Create owner
    let owner_keypair = Keypair::new();
    let owner_pubkey = owner_keypair.pubkey();

    // 8. Create compressible token account
    let token_account_pubkey = create_compressible_token_account(
        &mut rpc,
        CreateCompressibleTokenAccountInputs {
            owner: owner_pubkey,
            mint,
            num_prepaid_epochs: 2,
            payer: &payer,
            token_account_keypair: None,
            lamports_per_write: Some(100),
            token_account_version: TokenDataVersion::ShaFlat,
        },
    )
    .await
    .expect("Failed to create compressible token account");

    println!(
        "Created compressible token account: {}",
        token_account_pubkey
    );

    // 10. Verify tracker has the account
    println!("Tracker has {} accounts", tracker.len());
    assert_eq!(
        tracker.len(),
        1,
        "Tracker should have 1 account, but has {}",
        tracker.len()
    );

    // 11. Verify account details
    let accounts = tracker.get_compressible_accounts();
    println!("len {}", tracker.len());
    assert_eq!(
        accounts.len(),
        1,
        "Should have 1 compressible account, but have {}",
        accounts.len()
    );

    let account_state = &accounts[0];
    assert_eq!(
        account_state.pubkey, token_account_pubkey,
        "Pubkey mismatch"
    );
    assert_eq!(account_state.account.mint, mint.to_bytes(), "Mint mismatch");
    assert_eq!(
        account_state.account.owner,
        owner_pubkey.to_bytes(),
        "Owner mismatch"
    );
    assert_eq!(account_state.account.amount, 0, "Amount should be 0");
    assert!(account_state.lamports > 0, "Lamports should be > 0");
    let lamports = account_state.lamports;

    rpc.airdrop_lamports(&account_state.pubkey, 10_000_000)
        .await
        .expect("Failed to airdrop to token account");
    let accounts = tracker.get_compressible_accounts();
    let account_state = &accounts[0];
    assert_eq!(
        account_state.lamports,
        lamports + 10_000_000,
        "Lamports after airdrop mismatch"
    );

    println!("Account verification successful!");

    // 12. Test multiple accounts - create second account
    let owner_keypair_2 = Keypair::new();
    let token_account_pubkey_2 = create_compressible_token_account(
        &mut rpc,
        CreateCompressibleTokenAccountInputs {
            owner: owner_keypair_2.pubkey(),
            mint,
            num_prepaid_epochs: 3,
            payer: &payer,
            token_account_keypair: None,
            lamports_per_write: Some(100),
            token_account_version: TokenDataVersion::ShaFlat,
        },
    )
    .await
    .expect("Failed to create second compressible token account");

    println!(
        "Created second compressible token account: {}",
        token_account_pubkey_2
    );

    assert_eq!(
        tracker.len(),
        2,
        "Tracker should have 2 accounts after creating second account"
    );

    println!("Multiple account test successful!");

    // 13. Shutdown subscriber
    shutdown_tx
        .send(())
        .expect("Failed to send shutdown signal");

    // 14. Wait for subscriber to finish
    subscriber_handle.await.expect("Subscriber task panicked");

    println!("Test completed successfully!");
}
