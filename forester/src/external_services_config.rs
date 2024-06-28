#[derive(Debug, Clone)]
pub struct ExternalServicesConfig {
    pub rpc_url: String,
    pub ws_rpc_url: String,
    pub indexer_url: String,
    pub prover_url: String,
    pub derivation: String,
}

impl ExternalServicesConfig {
    pub fn local() -> Self {
        Self {
            rpc_url: "http://localhost:8899".to_string(),
            ws_rpc_url: "ws://localhost:8900".to_string(),
            indexer_url: "http://localhost:8784".to_string(),
            prover_url: "http://localhost:3001".to_string(),
            derivation: "En9a97stB3Ek2n6Ey3NJwCUJnmTzLMMEA5C69upGDuQP".to_string(),
        }
    }

    pub fn zktestnet() -> Self {
        Self {
            rpc_url: "https://zk-testnet.helius.dev:8899".to_string(),
            ws_rpc_url: "ws://zk-testnet.helius.dev:8900".to_string(),
            indexer_url: "https://zk-testnet.helius.dev:8784".to_string(),
            prover_url: "https://zk-testnet.helius.dev:3001".to_string(),
            derivation: "En9a97stB3Ek2n6Ey3NJwCUJnmTzLMMEA5C69upGDuQP".to_string(),
        }
    }
}
