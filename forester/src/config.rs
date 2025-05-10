use std::{str::FromStr, time::Duration};

use account_compression::utils::constants::{ADDRESS_QUEUE_VALUES, STATE_NULLIFIER_QUEUE_VALUES};
use anchor_lang::Id;
use forester_utils::forester_epoch::{Epoch, TreeAccounts, TreeForesterSchedule};
use light_client::rpc::RetryConfig;
use light_registry::{EpochPda, ForesterEpochPda};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};

use crate::{
    cli::{StartArgs, StatusArgs},
    errors::ConfigError,
    Result,
};

#[derive(Debug)]
pub struct ForesterConfig {
    pub external_services: ExternalServicesConfig,
    pub retry_config: RetryConfig,
    pub queue_config: QueueConfig,
    pub indexer_config: IndexerConfig,
    pub transaction_config: TransactionConfig,
    pub general_config: GeneralConfig,
    pub rpc_pool_config: RpcPoolConfig,
    pub registry_pubkey: Pubkey,
    pub payer_keypair: Keypair,
    pub derivation_pubkey: Pubkey,
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
    pub pagerduty_routing_key: Option<String>,
    pub rpc_rate_limit: Option<u32>,
    pub photon_rate_limit: Option<u32>,
    pub send_tx_rate_limit: Option<u32>,
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
    pub legacy_ixs_per_tx: usize,
    pub batch_ixs_per_tx: usize,
    pub max_concurrent_batches: usize,
    pub cu_limit: u32,
    pub enable_priority_fees: bool,
}

#[derive(Debug, Clone)]
pub struct GeneralConfig {
    pub slot_update_interval_seconds: u64,
    pub tree_discovery_interval_seconds: u64,
    pub enable_metrics: bool,
    pub skip_v1_state_trees: bool,
    pub skip_v1_address_trees: bool,
    pub skip_v2_state_trees: bool,
    pub skip_v2_address_trees: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        GeneralConfig {
            rpc_pool_size: 20,
            slot_update_interval_seconds: 10,
            tree_discovery_interval_seconds: 1,
            enable_metrics: true,
            skip_v1_state_trees: false,
            skip_v1_address_trees: false,
            skip_v2_state_trees: false,
            skip_v2_address_trees: false,
        }
    }
}

impl GeneralConfig {
    pub fn test_address_v2() -> Self {
        GeneralConfig {
            rpc_pool_size: 10,
            slot_update_interval_seconds: 10,
            tree_discovery_interval_seconds: 1,
            enable_metrics: true,
            skip_v1_state_trees: true,
            skip_v1_address_trees: true,
            skip_v2_state_trees: true,
            skip_v2_address_trees: false,
        }
    }

    pub fn test_state_v2() -> Self {
        GeneralConfig {
            rpc_pool_size: 10,
            slot_update_interval_seconds: 10,
            tree_discovery_interval_seconds: 1,
            enable_metrics: true,
            skip_v1_state_trees: true,
            skip_v1_address_trees: true,
            skip_v2_state_trees: false,
            skip_v2_address_trees: true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RpcPoolConfig {
    pub max_size: u32,
    pub connection_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_retries: u32,
    pub initial_retry_delay_ms: u64,
    pub max_retry_delay_ms: u64,
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
            legacy_ixs_per_tx: 1,
            batch_ixs_per_tx: 4,
            max_concurrent_batches: 20,
            cu_limit: 1_000_000,
            enable_priority_fees: false,
        }
    }
}
impl ForesterConfig {
    pub fn new_for_start(args: &StartArgs) -> Result<Self> {
        let registry_pubkey = light_registry::program::LightRegistry::id().to_string();

        let payer: Vec<u8> = match &args.payer {
            Some(payer_str) => {
                serde_json::from_str(payer_str).map_err(|e| ConfigError::JsonParse {
                    field: "payer",
                    error: e.to_string(),
                })?
            }
            None => return Err(ConfigError::MissingField { field: "payer" })?,
        };
        let payer =
            Keypair::from_bytes(&payer).map_err(|e| ConfigError::InvalidKeypair(e.to_string()))?;

        let derivation: Vec<u8> = match &args.derivation {
            Some(derivation_str) => {
                serde_json::from_str(derivation_str).map_err(|e| ConfigError::JsonParse {
                    field: "derivation",
                    error: e.to_string(),
                })?
            }
            None => {
                return Err(ConfigError::MissingField {
                    field: "derivation",
                })?
            }
        };
        let derivation_array: [u8; 32] =
            derivation
                .try_into()
                .map_err(|_| ConfigError::InvalidDerivation {
                    reason: "must be exactly 32 bytes".to_string(),
                })?;
        let derivation = Pubkey::from(derivation_array);

        let rpc_url = args
            .rpc_url
            .clone()
            .ok_or(ConfigError::MissingField { field: "rpc_url" })?;

        Ok(Self {
            external_services: ExternalServicesConfig {
                rpc_url,
                ws_rpc_url: args.ws_rpc_url.clone(),
                indexer_url: args.indexer_url.clone(),
                prover_url: args.prover_url.clone(),
                photon_api_key: args.photon_api_key.clone(),
                pushgateway_url: args.push_gateway_url.clone(),
                pagerduty_routing_key: args.pagerduty_routing_key.clone(),
                rpc_rate_limit: args.rpc_rate_limit,
                photon_rate_limit: args.photon_rate_limit,
                send_tx_rate_limit: args.send_tx_rate_limit,
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
                legacy_ixs_per_tx: args.legacy_ixs_per_tx,
                batch_ixs_per_tx: args.batch_ixs_per_tx,
                max_concurrent_batches: args.transaction_max_concurrent_batches,
                cu_limit: args.cu_limit,
                enable_priority_fees: args.enable_priority_fees,
            },
            general_config: GeneralConfig {
                slot_update_interval_seconds: args.slot_update_interval_seconds,
                tree_discovery_interval_seconds: args.tree_discovery_interval_seconds,
                enable_metrics: args.enable_metrics(),
                skip_v1_state_trees: false,
                skip_v2_state_trees: false,
                skip_v1_address_trees: false,
                skip_v2_address_trees: false,
            },
            rpc_pool_config: RpcPoolConfig {
                max_size: args.rpc_pool_size,
                connection_timeout_secs: args.rpc_pool_connection_timeout_secs,
                idle_timeout_secs: args.rpc_pool_idle_timeout_secs,
                max_retries: args.rpc_pool_max_retries,
                initial_retry_delay_ms: args.rpc_pool_initial_retry_delay_ms,
                max_retry_delay_ms: args.rpc_pool_max_retry_delay_ms,
            },
            registry_pubkey: Pubkey::from_str(&registry_pubkey).map_err(|e| {
                ConfigError::InvalidPubkey {
                    field: "registry_pubkey",
                    error: e.to_string(),
                }
            })?,
            payer_keypair: payer,
            derivation_pubkey: derivation,
            address_tree_data: vec![],
            state_tree_data: vec![],
        })
    }

    pub fn new_for_status(args: &StatusArgs) -> Result<Self> {
        let rpc_url = args.rpc_url.clone();

        Ok(Self {
            external_services: ExternalServicesConfig {
                rpc_url,
                ws_rpc_url: None,
                indexer_url: None,
                prover_url: None,
                photon_api_key: None,
                pushgateway_url: args.push_gateway_url.clone(),
                pagerduty_routing_key: args.pagerduty_routing_key.clone(),
                rpc_rate_limit: None,
                photon_rate_limit: None,
                send_tx_rate_limit: None,
            },
            retry_config: RetryConfig::default(),
            queue_config: QueueConfig::default(),
            indexer_config: IndexerConfig::default(),
            transaction_config: TransactionConfig::default(),
            general_config: GeneralConfig {
                slot_update_interval_seconds: 10,
                tree_discovery_interval_seconds: 60,
                enable_metrics: args.enable_metrics(),
                skip_v1_state_trees: false,
                skip_v2_state_trees: false,
                skip_v1_address_trees: false,
                skip_v2_address_trees: false,
            },
            rpc_pool_config: RpcPoolConfig {
                max_size: 10,
                connection_timeout_secs: 15,
                idle_timeout_secs: 300,
                max_retries: 10,
                initial_retry_delay_ms: 1000,
                max_retry_delay_ms: 16000,
            },
            registry_pubkey: Pubkey::default(),
            payer_keypair: Keypair::new(),
            derivation_pubkey: Pubkey::default(),
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
            rpc_pool_config: self.rpc_pool_config,
            registry_pubkey: self.registry_pubkey,
            payer_keypair: self.payer_keypair.insecure_clone(),
            derivation_pubkey: self.derivation_pubkey,
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
    pub fn add_trees_with_schedule(
        &mut self,
        trees: &[TreeAccounts],
        current_solana_slot: u64,
    ) -> Result<()> {
        for tree in trees {
            let tree_schedule = TreeForesterSchedule::new_with_schedule(
                tree,
                current_solana_slot,
                &self.forester_epoch_pda,
                &self.epoch_pda,
            )?;
            self.trees.push(tree_schedule);
        }
        Ok(())
    }
}
