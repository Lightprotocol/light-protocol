pub mod acp_sdk;
pub mod env_accounts;
pub mod env_accounts_v1;
pub mod indexer;
pub mod test_batch_forester;
pub mod test_env;
pub mod test_rpc;

pub mod prover {
    pub use light_prover_client::gnark::helpers::{
        kill_prover, ProofType, ProverConfig, ProverMode,
    };
}
#[cfg(feature = "devenv")]
pub const PHOTON_INDEXER_LOCAL_HOST_URL: &str = "http://127.0.0.1:8784";
#[cfg(not(feature = "devenv"))]
pub const PHOTON_INDEXER_LOCAL_HOST_URL: &str = "http://127.0.0.1:3001";
