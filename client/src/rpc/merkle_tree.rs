use std::mem;

use async_trait::async_trait;
use light_concurrent_merkle_tree::{
    copy::ConcurrentMerkleTreeCopy, errors::ConcurrentMerkleTreeError, light_hasher::Poseidon,
};
use light_indexed_merkle_tree::{copy::IndexedMerkleTreeCopy, errors::IndexedMerkleTreeError};
use light_sdk::state::MerkleTreeMetadata;
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

use super::{RpcConnection, RpcError};

#[derive(Error, Debug)]
pub enum MerkleTreeExtError {
    #[error(transparent)]
    Rpc(#[from] RpcError),

    #[error(transparent)]
    ConcurrentMerkleTree(#[from] ConcurrentMerkleTreeError),

    #[error(transparent)]
    IndexedMerkleTree(#[from] IndexedMerkleTreeError),
}

/// Extension to the RPC connection which provides convenience utilities for
/// fetching Merkle trees.
#[async_trait]
pub trait MerkleTreeExt: RpcConnection {
    async fn get_state_merkle_tree(
        &self,
        pubkey: Pubkey,
    ) -> Result<ConcurrentMerkleTreeCopy<Poseidon, 26>, MerkleTreeExtError> {
        let account = self.get_account(pubkey).await?.unwrap();
        let tree = ConcurrentMerkleTreeCopy::from_bytes_copy(
            &account.data[8 + mem::size_of::<MerkleTreeMetadata>()..],
        )?;

        Ok(tree)
    }

    async fn get_address_merkle_tree(
        &self,
        pubkey: Pubkey,
    ) -> Result<IndexedMerkleTreeCopy<Poseidon, usize, 26, 16>, MerkleTreeExtError> {
        let account = self.get_account(pubkey).await?.unwrap();
        let tree = IndexedMerkleTreeCopy::from_bytes_copy(
            &account.data[8 + mem::size_of::<MerkleTreeMetadata>()..],
        )?;

        Ok(tree)
    }
}
