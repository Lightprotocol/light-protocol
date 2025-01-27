use forester::{
    cli::StartArgs,
    send_transaction::{get_capped_priority_fee, request_priority_fee_estimate, CapConfig},
    ForesterConfig,
};
use light_client::rpc::{RpcConnection, SolanaRpcConnection};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signer};
use url::Url;

use crate::test_utils::init;
mod test_utils;

#[tokio::test]
#[ignore]
async fn test_priority_fee_request() {
    dotenvy::dotenv().ok();

    init(None).await;

    let args = StartArgs {
        rpc_url: Some(
            std::env::var("FORESTER_RPC_URL").expect("FORESTER_RPC_URL must be set in environment"),
        ),
        push_gateway_url: None,
        pagerduty_routing_key: None,
        ws_rpc_url: Some(
            std::env::var("FORESTER_WS_RPC_URL")
                .expect("FORESTER_WS_RPC_URL must be set in environment"),
        ),
        indexer_url: Some(
            std::env::var("FORESTER_INDEXER_URL")
                .expect("FORESTER_INDEXER_URL must be set in environment"),
        ),
        prover_url: Some(
            std::env::var("FORESTER_PROVER_URL")
                .expect("FORESTER_PROVER_URL must be set in environment"),
        ),
        payer: Some(
            std::env::var("FORESTER_PAYER").expect("FORESTER_PAYER must be set in environment"),
        ),
        derivation: Some(
            std::env::var("FORESTER_DERIVATION_PUBKEY")
                .expect("FORESTER_DERIVATION_PUBKEY must be set in environment"),
        ),
        photon_api_key: Some(
            std::env::var("PHOTON_API_KEY").expect("PHOTON_API_KEY must be set in environment"),
        ),
        indexer_batch_size: 50,
        indexer_max_concurrent_batches: 10,
        transaction_batch_size: 1,
        transaction_max_concurrent_batches: 20,
        cu_limit: 1_000_000,
        rpc_pool_size: 20,
        slot_update_interval_seconds: 10,
        tree_discovery_interval_seconds: 5,
        max_retries: 3,
        retry_delay: 1000,
        retry_timeout: 30000,
        state_queue_start_index: 0,
        state_queue_processing_length: 28807,
        address_queue_start_index: 0,
        address_queue_processing_length: 28807,
        rpc_rate_limit: None,
        photon_rate_limit: None,
        send_tx_rate_limit: None,
    };

    let config = ForesterConfig::new_for_start(&args).expect("Failed to create config");

    // Setup RPC connection using config
    let mut rpc = SolanaRpcConnection::new(
        config.external_services.rpc_url,
        Some(CommitmentConfig::confirmed()),
    );
    rpc.payer = config.payer_keypair.insecure_clone();

    let account_keys = vec![config.payer_keypair.pubkey()];

    let url = Url::parse(&rpc.get_url()).expect("Failed to parse URL");
    println!("URL: {}", url);
    let priority_fee = request_priority_fee_estimate(&url, account_keys)
        .await
        .unwrap();

    println!("Priority fee: {:?}", priority_fee);
    assert!(priority_fee > 0, "Priority fee should be greater than 0");
}
#[test]

fn test_capped_priority_fee() {
    let cap_config = CapConfig {
        rec_fee_microlamports_per_cu: 50_000,
        min_fee_lamports: 10_000,
        max_fee_lamports: 100_000,
        // 1_000_000 cu x 50_000 microlamports per cu = 50_000 lamports total
        compute_unit_limit: 1_000_000,
    };
    let expected = 50_000;

    let result = get_capped_priority_fee(cap_config);
    assert_eq!(
        result, expected,
        "Priority fee capping failed for input {}",
        cap_config.rec_fee_microlamports_per_cu
    );

    let cap_config = CapConfig {
        rec_fee_microlamports_per_cu: 10_000,
        min_fee_lamports: 10_000,
        max_fee_lamports: 100_000,
        compute_unit_limit: 1_000_000,
    };
    let expected = 10_000;
    let result = get_capped_priority_fee(cap_config);
    assert_eq!(
        result, expected,
        "Priority fee capping failed for input {}",
        cap_config.rec_fee_microlamports_per_cu
    );

    let cap_config = CapConfig {
        rec_fee_microlamports_per_cu: 100_000,
        min_fee_lamports: 10_000,
        max_fee_lamports: 100_000,
        compute_unit_limit: 1_000_000,
    };
    let expected = 100_000;
    let result = get_capped_priority_fee(cap_config);
    assert_eq!(
        result, expected,
        "Priority fee capping failed for input {}",
        cap_config.rec_fee_microlamports_per_cu
    );

    let cap_config = CapConfig {
        rec_fee_microlamports_per_cu: 10_000,
        min_fee_lamports: 20_000,
        max_fee_lamports: 100_000,
        compute_unit_limit: 1_000_000,
    };
    let expected = 20_000;
    let result = get_capped_priority_fee(cap_config);
    assert_eq!(
        result, expected,
        "Priority fee capping failed for input {}",
        cap_config.rec_fee_microlamports_per_cu
    );

    let cap_config = CapConfig {
        rec_fee_microlamports_per_cu: 200_000,
        min_fee_lamports: 10_000,
        max_fee_lamports: 100_000,
        compute_unit_limit: 1_000_000,
    };
    let expected = 100_000;
    let result = get_capped_priority_fee(cap_config);
    assert_eq!(
        result, expected,
        "Priority fee capping failed for input {}",
        cap_config.rec_fee_microlamports_per_cu
    );

    let cap_config = CapConfig {
        rec_fee_microlamports_per_cu: 10_000,
        min_fee_lamports: 0,
        max_fee_lamports: 0,
        compute_unit_limit: 1_000_000,
    };
    let expected = 0;
    let result = get_capped_priority_fee(cap_config);
    assert_eq!(
        result, expected,
        "Priority fee capping failed for input {}",
        cap_config.rec_fee_microlamports_per_cu
    );

    let cap_config = CapConfig {
        rec_fee_microlamports_per_cu: 10_000,
        min_fee_lamports: 10_000,
        max_fee_lamports: 0,
        compute_unit_limit: 1_000_000,
    };
    println!("expecting panic");
    let result = std::panic::catch_unwind(|| get_capped_priority_fee(cap_config));
    assert!(
        result.is_err(),
        "Expected panic for max fee less than min fee"
    );

    let cap_config = CapConfig {
        rec_fee_microlamports_per_cu: 10_000,
        min_fee_lamports: 50_000,
        max_fee_lamports: 50_000,
        compute_unit_limit: 1_000_000,
    };
    let expected = 50_000;
    let result = get_capped_priority_fee(cap_config);
    assert_eq!(
        result, expected,
        "Priority fee capping failed for input {}",
        cap_config.rec_fee_microlamports_per_cu
    );

    let cap_config = CapConfig {
        rec_fee_microlamports_per_cu: 100_000,
        min_fee_lamports: 50_000,
        max_fee_lamports: 50_000,
        compute_unit_limit: 1_000_000,
    };
    let expected = 50_000;
    let result = get_capped_priority_fee(cap_config);
    assert_eq!(
        result, expected,
        "Priority fee capping failed for input {}",
        cap_config.rec_fee_microlamports_per_cu
    );
}
