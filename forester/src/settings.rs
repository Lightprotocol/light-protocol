use crate::external_services_config::ExternalServicesConfig;
use crate::ForesterConfig;
use account_compression::initialize_address_merkle_tree::Pubkey;
use config::Config;
use solana_sdk::signature::{Keypair, Signer};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::{env, fmt};

const REGISTRY_PUBKEY: &str = "7Z9Yuy3HkBCc2Wf3xzMGnz6qpV4n7ciwcoEMGKqhAnj1";

pub enum SettingsKey {
    Payer,
    RpcUrl,
    WsRpcUrl,
    IndexerUrl,
    ProverUrl,
    BatchSize,
    MaxRetries,
    ConcurrencyLimit,
    CULimit,
    RpcPoolSize,
    PhotonApiKey,
}

impl Display for SettingsKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SettingsKey::Payer => "PAYER",
                SettingsKey::RpcUrl => "RPC_URL",
                SettingsKey::WsRpcUrl => "WS_RPC_URL",
                SettingsKey::IndexerUrl => "INDEXER_URL",
                SettingsKey::ProverUrl => "PROVER_URL",
                SettingsKey::PhotonApiKey => "PHOTON_API_KEY",
                SettingsKey::ConcurrencyLimit => "CONCURRENCY_LIMIT",
                SettingsKey::BatchSize => "PHOTON_BATCH_SIZE",
                SettingsKey::MaxRetries => "MAX_RETRIES",
                SettingsKey::CULimit => "CU_LIMIT",
                SettingsKey::RpcPoolSize => "RPC_POOL_SIZE",
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
    let _ = dotenvy::dotenv();
    let config_path = locate_config_file();

    let settings = Config::builder()
        .add_source(config::File::with_name(&config_path))
        .add_source(config::Environment::with_prefix("FORESTER"))
        .build()
        .unwrap();

    let registry_pubkey = REGISTRY_PUBKEY.to_string();

    let payer = settings
        .get_string(&SettingsKey::Payer.to_string())
        .unwrap();
    let payer: Vec<u8> = convert(&payer).unwrap();
    let payer = Keypair::from_bytes(&payer).unwrap();

    let rpc_url = settings
        .get_string(&SettingsKey::RpcUrl.to_string())
        .expect("RPC_URL not found in config file or environment variables");
    let ws_rpc_url = settings
        .get_string(&SettingsKey::WsRpcUrl.to_string())
        .expect("WS_RPC_URL not found in config file or environment variables");
    let indexer_url = settings
        .get_string(&SettingsKey::IndexerUrl.to_string())
        .expect("INDEXER_URL not found in config file or environment variables");
    let prover_url = settings
        .get_string(&SettingsKey::ProverUrl.to_string())
        .expect("PROVER_URL not found in config file or environment variables");
    let photon_api_key = settings
        .get_string(&SettingsKey::PhotonApiKey.to_string())
        .ok();
    let concurrency_limit = settings
        .get_int(&SettingsKey::ConcurrencyLimit.to_string())
        .expect("CONCURRENCY_LIMIT not found in config file or environment variables");
    let batch_size = settings
        .get_int(&SettingsKey::BatchSize.to_string())
        .expect("PHOTON_BATCH_SIZE not found in config file or environment variables");
    let max_retries = settings
        .get_int(&SettingsKey::MaxRetries.to_string())
        .expect("MAX_RETRIES not found in config file or environment variables");
    let cu_limit = settings.get_int(&SettingsKey::CULimit.to_string()).unwrap();
    let rpc_pool_size = settings.get_int(&SettingsKey::CULimit.to_string()).unwrap();
    ForesterConfig {
        external_services: ExternalServicesConfig {
            rpc_url,
            ws_rpc_url,
            indexer_url,
            prover_url,
            photon_api_key,
            derivation: payer.pubkey().to_string(),
        },
        registry_pubkey: Pubkey::from_str(&registry_pubkey).unwrap(),
        payer_keypair: payer,
        concurrency_limit: concurrency_limit as usize,
        batch_size: batch_size as usize,
        max_retries: max_retries as usize,
        cu_limit: cu_limit as u32,
        rpc_pool_size: rpc_pool_size as usize,
        address_tree_data: vec![],
        state_tree_data: vec![],
    }
}
