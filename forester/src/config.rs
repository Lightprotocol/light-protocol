use crate::cli::{StartArgs, StatusArgs};
use crate::errors::ForesterError;
use account_compression::initialize_address_merkle_tree::Pubkey;
use account_compression::utils::constants::{ADDRESS_QUEUE_VALUES, STATE_NULLIFIER_QUEUE_VALUES};
use anchor_lang::Id;
use forester_utils::forester_epoch::{Epoch, TreeAccounts, TreeForesterSchedule};
use light_client::rpc::RetryConfig;
use light_registry::{EpochPda, ForesterEpochPda};
use solana_sdk::signature::Keypair;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug)]
pub struct ForesterConfig {
    pub external_services: ExternalServicesConfig,
    pub retry_config: RetryConfig,
    pub queue_config: QueueConfig,
    pub indexer_config: IndexerConfig,
    pub transaction_config: TransactionConfig,
    pub general_config: GeneralConfig,
    pub registry_pubkey: Pubkey,
    pub payer_keypair: Keypair,
    pub address_tree_data: Vec<TreeAccounts>,
    pub state_tree_data: Vec<TreeAccounts>,
}

#[derive(Debug, Clone)]
pub struct ExternalServicesConfig {
    pub rpc_url: String,
    pub ws_rpc_url: Option<String>,
    pub indexer_url: Option<String>,
    pub prover_url: Option<String>,
    pub photon_api_key: Option<String>,
    pub pushgateway_url: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct QueueConfig {
    pub state_queue_start_index: u16,
    pub state_queue_length: u16,
    pub address_queue_start_index: u16,
    pub address_queue_length: u16,
}

#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub batch_size: usize,
    pub max_concurrent_batches: usize,
}

#[derive(Debug, Clone)]
pub struct TransactionConfig {
    pub batch_size: usize,
    pub max_concurrent_batches: usize,
    pub cu_limit: u32,
}

#[derive(Debug, Clone)]
pub struct GeneralConfig {
    pub rpc_pool_size: usize,
    pub slot_update_interval_seconds: u64,
    pub tree_discovery_interval_seconds: u64,
    pub enable_metrics: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        QueueConfig {
            state_queue_start_index: 0,
            state_queue_length: STATE_NULLIFIER_QUEUE_VALUES,
            address_queue_start_index: 0,
            address_queue_length: ADDRESS_QUEUE_VALUES,
        }
    }
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            max_concurrent_batches: 10,
        }
    }
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            batch_size: 1,
            max_concurrent_batches: 20,
            cu_limit: 1_000_000,
        }
    }
}
impl ForesterConfig {
    pub fn new_for_start(args: &StartArgs) -> Result<Self, ForesterError> {
        let registry_pubkey = light_registry::program::LightRegistry::id().to_string();

        let payer: Vec<u8> = serde_json::from_str(&args.payer)
            .map_err(|e| ForesterError::ConfigError(e.to_string()))?;
        let payer =
            Keypair::from_bytes(&payer).map_err(|e| ForesterError::ConfigError(e.to_string()))?;

        Ok(Self {
            external_services: ExternalServicesConfig {
                rpc_url: args.rpc_url.clone(),
                ws_rpc_url: Some(args.ws_rpc_url.clone()),
                indexer_url: Some(args.indexer_url.clone()),
                prover_url: Some(args.prover_url.clone()),
                photon_api_key: Some(args.photon_api_key.clone()),
                pushgateway_url: args.push_gateway_url.clone(),
            },
            retry_config: RetryConfig {
                max_retries: args.max_retries,
                retry_delay: Duration::from_millis(args.retry_delay),
                timeout: Duration::from_millis(args.retry_timeout),
            },
            queue_config: QueueConfig {
                state_queue_start_index: args.state_queue_start_index,
                state_queue_length: args.state_queue_processing_length,
                address_queue_start_index: args.address_queue_start_index,
                address_queue_length: args.address_queue_processing_length,
            },
            indexer_config: IndexerConfig {
                batch_size: args.indexer_batch_size,
                max_concurrent_batches: args.indexer_max_concurrent_batches,
            },
            transaction_config: TransactionConfig {
                batch_size: args.transaction_batch_size,
                max_concurrent_batches: args.transaction_max_concurrent_batches,
                cu_limit: args.cu_limit,
            },
            general_config: GeneralConfig {
                rpc_pool_size: args.rpc_pool_size,
                slot_update_interval_seconds: args.slot_update_interval_seconds,
                tree_discovery_interval_seconds: args.tree_discovery_interval_seconds,
                enable_metrics: args.enable_metrics(),
            },
            registry_pubkey: Pubkey::from_str(&registry_pubkey)
                .map_err(|e| ForesterError::ConfigError(e.to_string()))?,
            payer_keypair: payer,
            address_tree_data: vec![],
            state_tree_data: vec![],
        })
    }

    pub fn new_for_status(args: &StatusArgs) -> Result<Self, ForesterError> {
        Ok(Self {
            external_services: ExternalServicesConfig {
                rpc_url: args.rpc_url.clone(),
                ws_rpc_url: None,
                indexer_url: None,
                prover_url: None,
                photon_api_key: None,
                pushgateway_url: args.push_gateway_url.clone(),
            },
            retry_config: RetryConfig::default(),
            queue_config: QueueConfig::default(),
            indexer_config: IndexerConfig::default(),
            transaction_config: TransactionConfig::default(),
            general_config: GeneralConfig {
                rpc_pool_size: 1,
                slot_update_interval_seconds: 10,
                tree_discovery_interval_seconds: 5,
                enable_metrics: args.enable_metrics(),
            },
            registry_pubkey: Pubkey::default(),
            payer_keypair: Keypair::new(),
            address_tree_data: vec![],
            state_tree_data: vec![],
        })
    }
}
impl Clone for ForesterConfig {
    fn clone(&self) -> Self {
        ForesterConfig {
            external_services: self.external_services.clone(),
            retry_config: self.retry_config,
            queue_config: self.queue_config,
            indexer_config: self.indexer_config.clone(),
            transaction_config: self.transaction_config.clone(),
            general_config: self.general_config.clone(),
            registry_pubkey: self.registry_pubkey,
            payer_keypair: self.payer_keypair.insecure_clone(),
            address_tree_data: self.address_tree_data.clone(),
            state_tree_data: self.state_tree_data.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ForesterEpochInfo {
    pub epoch: Epoch,
    pub epoch_pda: EpochPda,
    pub forester_epoch_pda: ForesterEpochPda,
    pub trees: Vec<TreeForesterSchedule>,
}

impl ForesterEpochInfo {
    pub fn add_trees_with_schedule(&mut self, trees: &[TreeAccounts], current_solana_slot: u64) {
        for tree in trees {
            let tree_schedule = TreeForesterSchedule::new_with_schedule(
                tree,
                current_solana_slot,
                &self.forester_epoch_pda,
                &self.epoch_pda,
            );
            self.trees.push(tree_schedule);
        }
    }
}
