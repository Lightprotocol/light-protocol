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
    CULimit,
    RpcPoolSize,
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
                SettingsKey::CULimit => "CU_LIMIT",
                SettingsKey::RpcPoolSize => "RPC_POOL_SIZE",
            }
        )
    }
}
