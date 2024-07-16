use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use config::{Config, ConfigError, File};
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;


#[derive(Debug, Deserialize)]
pub struct UrlConfig {
    pub rpc_url: String,
    pub ws_rpc_url: String,
    pub indexer_url: String,
    pub prover_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ForesterConfig {
    pub default_environment: String,
    pub cu_limit: u32,
    pub rpc_pool_size: usize,
    pub concurrency_limit: usize,
    pub batch_size: usize,
    pub max_retries: usize,
    pub max_concurrent_batches: usize,
    pub state_merkle_tree_pubkey: String,
    pub nullifier_queue_pubkey: String,
    pub registry_pubkey: String,
    pub address_merkle_tree_pubkey: String,
    pub address_merkle_tree_queue_pubkey: String,
    pub urls: HashMap<String, UrlConfig>,
}

impl ForesterConfig {
    pub fn new(environment: Option<&str>) -> Result<Self, ConfigError> {
        let config_path = ForesterConfig::locate_config_file();
        let settings = Config::builder()
            .add_source(config::File::with_name(&config_path))
            .add_source(config::Environment::with_prefix("FORESTER"))
            .build()
            .unwrap();
        
        let env = environment.unwrap_or(&config.default_environment);
        if let Some(url_config) = config.urls.get(env) {
            config.urls.insert("current".to_string(), url_config.clone());
        } else {
            return Err(ConfigError::Message(format!("Environment '{}' not found in configuration", env)));
        }

        Ok(config)
    }

    pub fn get_current_urls(&self) -> &UrlConfig {
        self.urls.get("current").unwrap()
    }

    pub fn state_merkle_tree_pubkey(&self) -> Pubkey {
        Pubkey::from_str(&self.state_merkle_tree_pubkey).unwrap()
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

}
impl Clone for ForesterConfig {

    fn clone(&self) -> Self {
        Self {
            external_services: self.external_services.clone(),
            nullifier_queue_pubkey: self.nullifier_queue_pubkey,
            state_merkle_tree_pubkey: self.state_merkle_tree_pubkey,
            address_merkle_tree_pubkey: self.address_merkle_tree_pubkey,
            address_merkle_tree_queue_pubkey: self.address_merkle_tree_queue_pubkey,
            registry_pubkey: self.registry_pubkey,
            payer_keypair: Keypair::from_bytes(&self.payer_keypair.to_bytes()).unwrap(),
            cu_limit: self.cu_limit,
            concurrency_limit: self.concurrency_limit,
            batch_size: self.batch_size,
            max_retries: self.max_retries,
            rpc_pool_size: self.rpc_pool_size,
        }
    }
}
