use std::sync::Arc;

use anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL;
use log::{info, LevelFilter};
use solana_sdk::signature::{Keypair, Signer};
use account_compression::AddressMerkleTreeAccount;
use account_compression::initialize_address_merkle_tree::Pubkey;
use forester::external_services_config::ExternalServicesConfig;
use forester::nullifier::state::get_nullifier_queue;
use forester::utils::spawn_validator;
use forester::{nullify_addresses, ForesterConfig, init_rpc};
use forester::errors::ForesterError;
use light_hasher::Poseidon;
use light_test_utils::e2e_test_env::{E2ETestEnv, GeneralActionConfig, KeypairActionConfig};
use light_test_utils::get_indexed_merkle_tree;
use light_test_utils::indexer::TestIndexer;
use light_test_utils::rpc::rpc_connection::RpcConnection;
use light_test_utils::rpc::solana_rpc::SolanaRpcUrl;
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{get_test_env_accounts, REGISTRY_ID_TEST_KEYPAIR};

async fn init() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(LevelFilter::Info.to_string())).is_test(true)
        .try_init();

    spawn_validator(Default::default()).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn empty_address_tree_test() {
    init().await;
    let env_accounts = get_test_env_accounts();
    let registry_keypair = Keypair::from_bytes(&REGISTRY_ID_TEST_KEYPAIR).unwrap();
    let config = ForesterConfig {
        external_services: ExternalServicesConfig::local(),
        nullifier_queue_pubkey: env_accounts.nullifier_queue_pubkey,
        state_merkle_tree_pubkey: env_accounts.merkle_tree_pubkey,
        address_merkle_tree_pubkey: env_accounts.address_merkle_tree_pubkey,
        address_merkle_tree_queue_pubkey: env_accounts.address_merkle_tree_queue_pubkey,
        registry_pubkey: registry_keypair.pubkey(),
        payer_keypair: env_accounts.forester.insecure_clone(),
        concurrency_limit: 1,
        batch_size: 1,
        max_retries: 5,
        max_concurrent_batches: 5,
    };

    let mut rpc = SolanaRpcConnection::new(SolanaRpcUrl::Localnet, None);

    rpc.airdrop_lamports(&rpc.get_payer().pubkey(), LAMPORTS_PER_SOL * 1000)
        .await
        .unwrap();

    let mut env = E2ETestEnv::<500, SolanaRpcConnection>::new(
        rpc,
        &env_accounts,
        KeypairActionConfig {
            compress_sol: Some(1.0),
            decompress_sol: Some(0.5),
            transfer_sol: Some(1.0),
            create_address: Some(1.0),
            compress_spl: None,
            decompress_spl: None,
            mint_spl: None,
            transfer_spl: None,
            max_output_accounts: Some(3),
            fee_assert: true,
        },
        GeneralActionConfig {
            add_keypair: Some(1.0),
            create_state_mt: Some(0.0),
            create_address_mt: Some(1.0),
            nullify_compressed_accounts: Some(0.0),
            empty_address_queue: Some(0.0),
        },
        0,
        None,
    )
        .await;

    let config = Arc::new(config.clone());
    let rpc = init_rpc(&config).await;
    let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

    let indexer = Arc::new(tokio::sync::Mutex::new(env.indexer.clone()));


    for _ in 0..10 {
        env.create_address(None).await;
    }

    assert_ne!(get_address_queue_length(&config, &mut env.rpc).await, 0);
    info!(
        "Address merkle tree: nullifying queue of {} accounts...",
        get_address_queue_length(&config, &mut env.rpc).await
    );

    nullify_addresses(config.clone(), rpc, indexer).await;
    assert_eq!(get_address_queue_length(&config, &mut env.rpc).await, 0);
}

async fn get_address_queue_length<R: RpcConnection>(config: &ForesterConfig, rpc: &mut R) -> usize {
    let queue = get_nullifier_queue(&config.address_merkle_tree_queue_pubkey, rpc)
        .await
        .unwrap();
    queue.len()
}
pub async fn get_changelog_indices<R: RpcConnection>(
    merkle_tree_pubkey: &Pubkey,
    client: &mut R,
) -> Result<(usize, usize), ForesterError> {
    let merkle_tree =
        get_indexed_merkle_tree::<AddressMerkleTreeAccount, R, Poseidon, usize, 26, 16>(
            client,
            *merkle_tree_pubkey,
        )
            .await;
    let changelog_index = merkle_tree.changelog_index();
    let indexed_changelog_index = merkle_tree.indexed_changelog_index();
    Ok((changelog_index, indexed_changelog_index))
}
