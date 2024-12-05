pub mod errors;
pub mod merkle_tree;
pub mod rpc_connection;
pub mod solana_rpc;

pub use errors::{assert_rpc_error, RpcError};
pub use light_compressed_token::TokenData;
pub use light_system_program::sdk::compressed_account::CompressedAccountWithMerkleContext;
use light_verifier::CompressedProof;
pub use rpc_connection::RpcConnection;
pub use solana_rpc::{RetryConfig, SolanaRpcConnection};

#[derive(Debug, Default)]
pub struct BatchedTreeProofRpcResult {
    pub proof: Option<CompressedProof>,
    // If none -> proof by index, else included in zkp
    pub root_indices: Vec<Option<u16>>,
    pub address_root_indices: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct TokenDataWithContext {
    pub token_data: TokenData,
    pub compressed_account: CompressedAccountWithMerkleContext,
}
