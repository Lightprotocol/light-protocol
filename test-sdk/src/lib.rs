use std::process::Stdio;

use cargo_metadata::MetadataCommand;
use light_macros::pubkey;
use light_sdk::merkle_context::AddressMerkleContext;
use photon_api::apis::configuration::Configuration;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use tokio::process::Command;

/// A test environment for programs using Light Protocol which manages:
///
/// - test validator
/// - Photon indexer
pub struct LightTest {
    payer: Keypair,
    pub photon_configuration: Configuration,
}

impl LightTest {
    /// Creates a new test environment.
    pub async fn new(program_name: &str) -> Self {
        let validator = Command::new("light")
            .arg("test-validator")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // .spawn()
            .status()
            .await
            .unwrap();
        println!("{validator:?}");
        if !validator.success() {
            panic!("failed to run the validator");
        }

        let payer = Keypair::new();

        let metadata = MetadataCommand::new().no_deps().exec().unwrap();
        let workspace_root = metadata.workspace_root;

        let program_so_path = format!("{workspace_root}/target/deploy/{program_name}.so");
        let program_keypair_path =
            format!("{workspace_root}/target/deploy/{program_name}-keypair.json");

        let out = Command::new("solana")
            .arg("program")
            .arg("deploy")
            .arg(program_so_path)
            .arg("--program-id")
            .arg(program_keypair_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .await
            .unwrap();
        println!("{out:?}");

        let photon_configuration = Configuration {
            base_path: "http://localhost:8784".to_string(),
            api_key: None,
            ..Default::default()
        };

        Self {
            payer,
            photon_configuration,
        }
    }

    /// Returns a Solana RPC client connected to the test validator.
    pub fn client(&self) -> RpcClient {
        RpcClient::new("http://localhost:8899")
    }

    pub fn address_merkle_context(&self) -> AddressMerkleContext {
        AddressMerkleContext {
            address_merkle_tree_pubkey: pubkey!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2"),
            address_queue_pubkey: pubkey!("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F"),
        }
    }

    /// Returns a payer keypair.
    pub fn payer(&self) -> Keypair {
        self.payer.insecure_clone()
    }
}
