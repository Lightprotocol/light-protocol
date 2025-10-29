//! # Light Program Test
//!
//! A fast local test environment for Solana programs using compressed accounts and tokens.
//!
//! For Rust and Anchor program development, see [`light-sdk`](https://docs.rs/light-sdk).
//! For Rust client, see [`light-client`](https://docs.rs/light-client).
//! For full program examples, see the [Program Examples](https://github.com/Lightprotocol/program-examples).
//! For detailed documentation, visit [zkcompression.com](https://www.zkcompression.com/).
//!
//! # Features
//!
//! - `v2` - Enables v2 batched Merkle trees.
//!
//! ## Testing Features
//! - Fast in-memory indexer and SVM via [LiteSVM](https://github.com/LiteSVM/LiteSVM)
//! - Supports custom programs
//! - Prover server via [Light CLI](https://www.npmjs.com/package/@lightprotocol/zk-compression-cli)

#![allow(deprecated)]
#![allow(clippy::result_large_err)]
//!
//! **Use `light-program-test` when:**
//! - You need fast test execution
//! - You write unit/integration tests for your program or client code
//!
//! **Use `solana-test-validator` when:**
//! - You need RPC methods or external tools that are incompatible with LiteSVM
//! - Testing against real validator behavior
//!
//! ## Configuration Options
//!
//! ### `with_prover: bool`
//! - `true`: Starts a prover server in the background for generating validity proofs
//! - `false`: Runs without prover (faster for tests that don't need proofs, or repeated test runs to reduce startup time)
//!
//! ### `additional_programs: Option<Vec<(&str, Pubkey)>>`
//! - Specify custom programs to deploy alongside the default Light Protocol programs
//! - Format: `vec![("program_name", program_id)]`
//! - Programs are loaded from built artifacts
//!
//! ## Prerequisites
//!
//! 1. **ZK Compression CLI**: Required to start the prover server and download Light Protocol programs
//!    ```bash
//!    npm i -g @lightprotocol/zk-compression-cli
//!    ```
//!    - If programs are missing after CLI installation, run `light test-validator` once to download them
//!
//! 2. **Build programs**: Run `cargo test-sbf` to build program binaries and set the required
//!    environment variables for locating program artifacts
//!
//! ## Prover Server
//!
//! The prover server runs on port 3001 when enabled. To manually stop it:
//! ```bash
//! # Find the process ID
//! lsof -i:3001
//! # Kill the process
//! kill <pid>
//! ```
//!
//! ## Debugging
//!
//! Set `RUST_BACKTRACE=1` to show detailed transaction information including accounts and parsed instructions:
//! ```bash
//! RUST_BACKTRACE=1 cargo test-sbf -- --nocapture
//! ```
//!
//! ## Examples
//!
//! ### V1 Trees
//! ```rust
//! use light_program_test::{LightProgramTest, ProgramTestConfig};
//! use solana_sdk::signer::Signer;
//!
//! #[tokio::test]
//! async fn test_v1_compressed_account() {
//!     // Initialize with v1 trees
//!     let config = ProgramTestConfig::default();
//!     let mut rpc = LightProgramTest::new(config).await.unwrap();
//!
//!     let payer = Keypair::new();
//!
//!     // Get v1 tree info
//!     let address_tree_info = rpc.get_address_tree_v1();
//!     let state_tree_info = rpc.get_random_state_tree_info();
//!
//!     // Airdrop for testing
//!     rpc.airdrop_lamports(&payer.pubkey(), 1_000_000_000).await.unwrap();
//!
//!     // Query compressed accounts using Indexer trait
//!     let accounts = rpc.indexer().unwrap()
//!         .get_compressed_accounts_by_owner(&payer.pubkey())
//!         .await
//!         .unwrap();
//!
//!     println!("Found {} compressed accounts", accounts.len());
//! }
//! ```
//!
//! ### V2 Trees
//! ```rust
//! use light_program_test::{LightProgramTest, ProgramTestConfig};
//! use solana_sdk::signer::Signer;
//!
//! #[tokio::test]
//! async fn test_v2_compressed_account() {
//!     // Initialize with v2 batched trees and custom program
//!     let config = ProgramTestConfig::new_v2(
//!         true, // with_prover
//!         Some(vec![("my_program", my_program::ID)])
//!     );
//!     let mut rpc = LightProgramTest::new(config).await.unwrap();
//!
//!     let payer = Keypair::new();
//!
//!     // Get v2 tree pubkeys
//!     let address_tree_info = rpc.get_address_tree_v2();
//!     let state_tree_info = rpc.get_random_state_tree_info();
//!
//!
//!     rpc.airdrop_lamports(&payer.pubkey(), 1_000_000_000).await.unwrap();
//!
//!     // Query using Indexer trait methods
//!     let accounts = rpc.indexer().unwrap()
//!         .get_compressed_accounts_by_owner(&payer.pubkey())
//!         .await
//!         .unwrap();
//!
//!     println!("Found {} compressed accounts with v2 trees", accounts.len());
//! }
//! ```

pub mod accounts;
pub mod compressible;
#[cfg(feature = "devenv")]
pub mod forester;
pub mod indexer;
pub mod litesvm_extensions;
pub mod logging;
pub mod program_test;
pub mod utils;

pub use light_client::{
    indexer::{AddressWithTree, Indexer},
    rpc::{Rpc, RpcError},
};
pub use litesvm_extensions::LiteSvmExtensions;
pub use program_test::{config::ProgramTestConfig, LightProgramTest};
