use clap::Parser;
use env_logger::Env;
use forester::cli::{Cli, Commands};
use forester::nqmt::reindex_and_store;
use forester::{
    init_config, init_rpc, nullify_addresses, nullify_state, subscribe_state, ForesterConfig,
};
use light_registry::sdk::get_group_pda;
use light_test_utils::indexer::{AddressMerkleTreeAccounts, StateMerkleTreeAccounts, TestIndexer};
use light_test_utils::rpc::SolanaRpcConnection;
use light_test_utils::test_env::{GROUP_PDA_SEED_TEST_KEYPAIR, SIGNATURE_CPI_TEST_KEYPAIR};
use log::{debug, error};
use solana_sdk::signature::{Keypair, Signer};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let config: Arc<ForesterConfig> = Arc::new(init_config());
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Subscribe) => {
            debug!(
                "Subscribe to nullify compressed accounts for indexed array: {} and merkle tree: {}",
                config.nullifier_queue_pubkey, config.state_merkle_tree_pubkey
            );
            subscribe_state(config.clone()).await;
        }
        Some(Commands::NullifyState) => {
            nullify_state(config).await;
        }
        Some(Commands::NullifyAddresses) => {
            run_nullify_addresses(config).await;
        }
        Some(Commands::Nullify) => {
            let state_nullifier = tokio::spawn(nullify_state(config.clone()));
            let address_nullifier = tokio::spawn(run_nullify_addresses(config));

            // Wait for both nullifiers to complete
            let (state_result, address_result) = tokio::join!(state_nullifier, address_nullifier);

            if let Err(e) = state_result {
                error!("State nullifier encountered an error: {:?}", e);
            }

            if let Err(e) = address_result {
                error!("Address nullifier encountered an error: {:?}", e);
            }

            debug!("All nullification processes completed");
        }
        Some(Commands::Index) => {
            debug!("Reindex merkle tree & nullifier queue accounts");
            debug!(
                "Initial merkle tree account: {}",
                config.state_merkle_tree_pubkey
            );
            let _ = reindex_and_store(&config);
        }
        None => {
            return;
        }
    }
}

async fn run_nullify_addresses(config: Arc<ForesterConfig>) {
    let rpc = init_rpc(&config).await;
    let rpc = Arc::new(tokio::sync::Mutex::new(rpc));

    let cpi_context_account_keypair = Keypair::from_bytes(&SIGNATURE_CPI_TEST_KEYPAIR).unwrap();
    let group_seed_keypair = Keypair::from_bytes(&GROUP_PDA_SEED_TEST_KEYPAIR).unwrap();
    let group_pda = get_group_pda(group_seed_keypair.pubkey());

    let indexer: TestIndexer<200, SolanaRpcConnection> = TestIndexer::new(
        vec![StateMerkleTreeAccounts {
            merkle_tree: config.state_merkle_tree_pubkey,
            nullifier_queue: config.nullifier_queue_pubkey,
            cpi_context: cpi_context_account_keypair.pubkey(),
        }],
        vec![AddressMerkleTreeAccounts {
            merkle_tree: config.address_merkle_tree_pubkey,
            queue: config.address_merkle_tree_queue_pubkey,
        }],
        config.payer_keypair.insecure_clone(),
        group_pda,
        true,
        true,
    )
    .await;
    let indexer = Arc::new(tokio::sync::Mutex::new(indexer));
    nullify_addresses(config.clone(), rpc, indexer).await;
}
