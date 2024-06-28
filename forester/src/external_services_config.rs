#[derive(Debug, Clone)]
pub struct ExternalServicesConfig {
    pub rpc_url: String,
    pub ws_rpc_url: String,
    pub indexer_url: String,
    pub prover_url: String,
    pub derivation: String,
}
