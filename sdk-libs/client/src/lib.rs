//! # Light Client
//!
//! A client library for interacting with Light Protocol compressed accounts and RPC endpoints.
//!
//! ## Features
//! - Connect to various RPC endpoints (local test validator, devnet/mainnet)
//! - Query compressed accounts and validity proofs from RPC endpoints
//! - Support for both v1 and v2 merkle trees (with v2 feature)
//! - Start local test validator with Light Protocol programs
//!
//! ## Prerequisites
//!
//! For local test validator usage, install the Light CLI:
//! ```bash
//! npm i -g @lightprotocol/zk-compression-cli
//! ```
//!
//! ## Example
//!
//! ```no_run
//! use light_client::{
//!     rpc::{LightClient, RpcConfig, Rpc},
//!     indexer::{Indexer, IndexerRpcConfig, RetryConfig},
//!     local_test_validator::{spawn_validator, LightValidatorConfig},
//! };
//! use light_prover_client::prover::ProverConfig;
//! use solana_pubkey::Pubkey;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Start local test validator with Light Protocol programs
//!     let config = LightValidatorConfig {
//!         enable_indexer: true,
//!         prover_config: Some(ProverConfig::default()),
//!         wait_time: 75,
//!         sbf_programs: vec![],
//!         limit_ledger_size: None,
//!     };
//!     spawn_validator(config).await;
//!
//!     // Connect to the validator
//!     let mut rpc = LightClient::new(RpcConfig::local()).await?;
//!
//!     // Or connect to devnet/mainnet:
//!     // let mut rpc = LightClient::new(RpcConfig::new("https://devnet.helius-rpc.com/?api-key=YOUR_KEY")).await?;
//!     // let mut rpc = LightClient::new(RpcConfig::new("https://mainnet.helius-rpc.com/?api-key=YOUR_KEY")).await?;
//!
//!     let owner = Pubkey::new_unique();
//!
//!     // Create indexer config for queries
//!     let slot = rpc.get_slot().await?;
//!     let config = IndexerRpcConfig {
//!         slot,
//!         retry_config: RetryConfig::default(),
//!     };
//!
//!     // Query compressed accounts using Indexer trait
//!     let accounts = rpc
//!         .get_compressed_accounts_by_owner(&owner, None, Some(config))
//!         .await?;
//!
//!     println!("Found {} compressed accounts", accounts.value.items.len());
//!
//!     // Get validity proofs for creating transactions
//!     let rpc_result = rpc
//!         .get_validity_proof(
//!             vec![], // add account hashes here
//!             vec![], // add addresses with address tree here
//!             None
//!         )
//!         .await?;
//!
//!     println!("Got validity proof and proof inputs {:?}", rpc_result.value);
//!
//!     Ok(())
//! }
//! ```

pub mod constants;
pub mod fee;
pub mod indexer;
pub mod local_test_validator;
pub mod rpc;

/// Reexport for ProverConfig and other types.
pub use light_prover_client;
