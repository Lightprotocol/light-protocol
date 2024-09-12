use std::{fmt::Debug, future::Future};

use light_sdk::{
    address::AddressWithMerkleContext,
    compressed_account::CompressedAccountWithMerkleContext,
    proof::{MerkleProof, NewAddressProofWithContext, ProofRpcResult},
};
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

pub mod photon;

#[derive(Error, Debug)]
pub enum IndexerError {
    // #[error("RPC Error: {0}")]
    // RpcError(#[from] solana_client::client_error::ClientError),
    // #[error("failed to deserialize account data")]
    // DeserializeError(#[from] solana_sdk::program_error::ProgramError),
    // #[error(transparent)]
    // HashSetError(#[from] HashSetError),
    // #[error(transparent)]
    // PhotonApiError(PhotonApiErrorWrapper),
    // #[error("error: {0:?}")]
    // Custom(String),
    #[error("unknown error")]
    Unknown,

    #[error("indexer returned an empty result")]
    EmptyResult,

    #[error("failed to hash a compressed account")]
    AccountHash,
}

/// Format of hashes.
///
/// Depending on the context, it's better to treat hashes either as arrays or
/// as strings.
///
/// Photon API takes hashes as strings.
///
/// In Solana program tests it's more convenient to operate on arrays. The
/// `Array` variant is being converted to strings by indexer implementations,
/// so the conversion doesn't have to be done independently in tests.
///
/// In forester, which only uses Photon, it makes more sense to just use
/// strings and avoid conversions.
#[derive(Debug)]
pub enum Hashes<'a> {
    Array(&'a [[u8; 32]]),
    String(&'a [String]),
}

pub trait Indexer: Sync + Send + Debug + 'static {
    fn get_compressed_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> impl Future<Output = Result<Vec<CompressedAccountWithMerkleContext>, IndexerError>> + Send + Sync;

    fn get_multiple_compressed_account_proofs<'a>(
        &self,
        hashes: Hashes<'a>,
    ) -> impl Future<Output = Result<Vec<MerkleProof>, IndexerError>> + Send + Sync;

    fn get_multiple_new_address_proofs(
        &self,
        merkle_tree_pubkey: &Pubkey,
        addresses: &[[u8; 32]],
    ) -> impl Future<Output = Result<Vec<NewAddressProofWithContext>, IndexerError>> + Send + Sync;

    fn get_validity_proof(
        &self,
        compressed_accounts: &[CompressedAccountWithMerkleContext],
        new_addresses: &[AddressWithMerkleContext],
    ) -> impl Future<Output = Result<ProofRpcResult, IndexerError>> + Send + Sync;
}
