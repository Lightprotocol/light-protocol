use crate::external_services_config::ExternalServicesConfig;
use crate::ForesterConfig;
use account_compression::initialize_address_merkle_tree::Pubkey;
use config::Config;
use solana_sdk::signature::{Keypair, Signer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::{env, fmt};

pub enum SettingsKey {
    Payer,
    StateMerkleTreePubkey,
    NullifierQueuePubkey,
    AddressMerkleTreePubkey,
    AddressMerkleTreeQueuePubkey,
    RegistryPubkey,
    RpcUrl,
    WsRpcUrl,
    IndexerUrl,
    ProverUrl,
    BatchSize,
    MaxRetries,
    ConcurrencyLimit,
}

impl Display for SettingsKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SettingsKey::Payer => "PAYER",
                SettingsKey::StateMerkleTreePubkey => "STATE_MERKLE_TREE_PUBKEY",
                SettingsKey::NullifierQueuePubkey => "NULLIFIER_QUEUE_PUBKEY",
                SettingsKey::RegistryPubkey => "REGISTRY_PUBKEY",
                SettingsKey::AddressMerkleTreePubkey => "ADDRESS_MERKLE_TREE_PUBKEY",
                SettingsKey::AddressMerkleTreeQueuePubkey => "ADDRESS_MERKLE_TREE_QUEUE_PUBKEY",
                SettingsKey::RpcUrl => "RPC_URL",
                SettingsKey::WsRpcUrl => "WS_RPC_URL",
                SettingsKey::IndexerUrl => "INDEXER_URL",
                SettingsKey::ProverUrl => "PROVER_URL",
                SettingsKey::ConcurrencyLimit => "CONCURRENCY_LIMIT",
                SettingsKey::BatchSize => "BATCH_SIZE",
                SettingsKey::MaxRetries => "MAX_RETRIES",
            }
        )
    }
}

fn locate_config_file() -> String {
    let file_name = "forester.toml";

    let exe_path = env::current_exe().unwrap();
    let exe_dir = exe_path.parent().unwrap();
    let config_path = exe_dir.join(file_name);
    if config_path.exists() {
        return config_path.to_str().unwrap().to_string();
    }

    file_name.to_string()
}

fn convert(json: &str) -> serde_json::Result<Vec<u8>> {
    serde_json::from_str(json)
}

pub fn init_config() -> ForesterConfig {
    let config_path = locate_config_file();

    let settings = Config::builder()
        .add_source(config::File::with_name(&config_path))
        .add_source(config::Environment::with_prefix("FORESTER"))
        .build()
        .unwrap();

    let state_merkle_tree_pubkey = settings
        .get_string(&SettingsKey::StateMerkleTreePubkey.to_string())
        .unwrap();
    let nullifier_queue_pubkey = settings
        .get_string(&SettingsKey::NullifierQueuePubkey.to_string())
        .unwrap();
    let address_merkle_tree_pubkey = settings
        .get_string(&SettingsKey::AddressMerkleTreePubkey.to_string())
        .unwrap();
    let address_merkle_tree_queue_pubkey = settings
        .get_string(&SettingsKey::AddressMerkleTreeQueuePubkey.to_string())
        .unwrap();
    let registry_pubkey = settings
        .get_string(&SettingsKey::RegistryPubkey.to_string())
        .unwrap();
    let payer = settings
        .get_string(&SettingsKey::Payer.to_string())
        .unwrap();
    let payer: Vec<u8> = convert(&payer).unwrap();
    let payer = Keypair::from_bytes(&payer).unwrap();

    let rpc_url = settings
        .get_string(&SettingsKey::RpcUrl.to_string())
        .unwrap();
    let ws_rpc_url = settings
        .get_string(&SettingsKey::WsRpcUrl.to_string())
        .unwrap();
    let indexer_url = settings
        .get_string(&SettingsKey::IndexerUrl.to_string())
        .unwrap();
    let prover_url = settings
        .get_string(&SettingsKey::ProverUrl.to_string())
        .unwrap();
    let concurrency_limit = settings
        .get_int(&SettingsKey::ConcurrencyLimit.to_string())
        .unwrap();
    let batch_size = settings
        .get_int(&SettingsKey::BatchSize.to_string())
        .unwrap();
    let max_retries = settings
        .get_int(&SettingsKey::MaxRetries.to_string())
        .unwrap();

    ForesterConfig {
        external_services: ExternalServicesConfig {
            rpc_url,
            ws_rpc_url,
            indexer_url,
            prover_url,
            derivation: payer.pubkey().to_string(),
        },
        nullifier_queue_pubkey: Pubkey::from_str(&nullifier_queue_pubkey).unwrap(),
        state_merkle_tree_pubkey: Pubkey::from_str(&state_merkle_tree_pubkey).unwrap(),
        address_merkle_tree_pubkey: Pubkey::from_str(&address_merkle_tree_pubkey).unwrap(),
        address_merkle_tree_queue_pubkey: Pubkey::from_str(&address_merkle_tree_queue_pubkey)
            .unwrap(),
        registry_pubkey: Pubkey::from_str(&registry_pubkey).unwrap(),
        payer_keypair: payer,
        concurrency_limit: concurrency_limit as usize,
        batch_size: batch_size as usize,
        max_retries: max_retries as usize,
    }
}
