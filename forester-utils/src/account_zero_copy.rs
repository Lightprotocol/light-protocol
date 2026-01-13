use std::{fmt, marker::PhantomData, mem, pin::Pin};

use light_client::rpc::Rpc;
use light_concurrent_merkle_tree::{
    copy::ConcurrentMerkleTreeCopy, errors::ConcurrentMerkleTreeError,
};
use light_hash_set::HashSet;
use light_hasher::Hasher;
use light_indexed_merkle_tree::{copy::IndexedMerkleTreeCopy, errors::IndexedMerkleTreeError};
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};
use solana_sdk::{account::Account, pubkey::Pubkey};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AccountZeroCopyError {
    #[error("RPC error: {0}")]
    RpcError(String),
    #[error("Account not found: {0}")]
    AccountNotFound(Pubkey),
}

#[derive(Debug, Clone)]
pub struct AccountZeroCopy<'a, T> {
    pub account: Pin<Box<Account>>,
    deserialized: *const T,
    _phantom_data: PhantomData<&'a T>,
}

impl<'a, T> AccountZeroCopy<'a, T> {
    pub async fn new<R: Rpc>(
        rpc: &mut R,
        address: Pubkey,
    ) -> Result<AccountZeroCopy<'a, T>, AccountZeroCopyError> {
        let account = rpc
            .get_account(address)
            .await
            .map_err(|e| AccountZeroCopyError::RpcError(e.to_string()))?
            .ok_or(AccountZeroCopyError::AccountNotFound(address))?;
        let account = Box::pin(account);
        let deserialized = account.data[8..].as_ptr() as *const T;

        Ok(Self {
            account,
            deserialized,
            _phantom_data: PhantomData,
        })
    }

    // Safe method to access `deserialized` ensuring the lifetime is respected
    pub fn deserialized(&self) -> &'a T {
        unsafe { &*self.deserialized }
    }
}

/// Fetches the given account, then copies and serializes it as a `HashSet`.
///
/// # Safety
///
/// This is highly unsafe. Ensuring that:
///
/// * The correct account is used.
/// * The account has enough space to be treated as a HashSet with specified
///   parameters.
/// * The account data is aligned.
///
/// Is the caller's responsibility.
pub async unsafe fn get_hash_set<T, R: Rpc>(
    rpc: &mut R,
    pubkey: Pubkey,
) -> Result<HashSet, AccountZeroCopyError> {
    let account = rpc
        .get_account(pubkey)
        .await
        .map_err(|e| AccountZeroCopyError::RpcError(e.to_string()))?
        .ok_or(AccountZeroCopyError::AccountNotFound(pubkey))?;
    let mut data = account.data.clone();

    HashSet::from_bytes_copy(&mut data[8 + mem::size_of::<T>()..])
        .map_err(|e| AccountZeroCopyError::RpcError(format!("HashSet parse error: {:?}", e)))
}

/// Fetches the given account, then copies and serializes it as a
/// `ConcurrentMerkleTree`.
pub async fn get_concurrent_merkle_tree<T, R, H, const HEIGHT: usize>(
    rpc: &mut R,
    pubkey: Pubkey,
) -> Result<ConcurrentMerkleTreeCopy<H, HEIGHT>, AccountZeroCopyError>
where
    R: Rpc,
    H: Hasher,
{
    let account = rpc
        .get_account(pubkey)
        .await
        .map_err(|e| AccountZeroCopyError::RpcError(e.to_string()))?
        .ok_or(AccountZeroCopyError::AccountNotFound(pubkey))?;

    ConcurrentMerkleTreeCopy::from_bytes_copy(&account.data[8 + mem::size_of::<T>()..]).map_err(
        |e| AccountZeroCopyError::RpcError(format!("ConcurrentMerkleTree parse error: {:?}", e)),
    )
}
// TODO: do discriminator check
/// Fetches the given account, then copies and serializes it as an
/// `IndexedMerkleTree`.
pub async fn get_indexed_merkle_tree<T, R, H, I, const HEIGHT: usize, const NET_HEIGHT: usize>(
    rpc: &mut R,
    pubkey: Pubkey,
) -> Result<IndexedMerkleTreeCopy<H, I, HEIGHT, NET_HEIGHT>, AccountZeroCopyError>
where
    R: Rpc,
    H: Hasher,
    I: CheckedAdd
        + CheckedSub
        + Copy
        + Clone
        + fmt::Debug
        + PartialOrd
        + ToBytes
        + TryFrom<usize>
        + Unsigned,
    usize: From<I>,
{
    let account = rpc
        .get_account(pubkey)
        .await
        .map_err(|e| AccountZeroCopyError::RpcError(e.to_string()))?
        .ok_or(AccountZeroCopyError::AccountNotFound(pubkey))?;

    IndexedMerkleTreeCopy::from_bytes_copy(&account.data[8 + mem::size_of::<T>()..]).map_err(|e| {
        AccountZeroCopyError::RpcError(format!("IndexedMerkleTree parse error: {:?}", e))
    })
}

/// Parse ConcurrentMerkleTree from raw account data bytes.
pub fn parse_concurrent_merkle_tree_from_bytes<T, H, const HEIGHT: usize>(
    data: &[u8],
) -> Result<ConcurrentMerkleTreeCopy<H, HEIGHT>, ConcurrentMerkleTreeError>
where
    H: Hasher,
{
    let offset = 8 + mem::size_of::<T>();
    if data.len() <= offset {
        return Err(ConcurrentMerkleTreeError::BufferSize(offset, data.len()));
    }
    ConcurrentMerkleTreeCopy::from_bytes_copy(&data[offset..])
}

/// Parse IndexedMerkleTree from raw account data byte
pub fn parse_indexed_merkle_tree_from_bytes<T, H, I, const HEIGHT: usize, const NET_HEIGHT: usize>(
    data: &[u8],
) -> Result<IndexedMerkleTreeCopy<H, I, HEIGHT, NET_HEIGHT>, IndexedMerkleTreeError>
where
    H: Hasher,
    I: CheckedAdd
        + CheckedSub
        + Copy
        + Clone
        + fmt::Debug
        + PartialOrd
        + ToBytes
        + TryFrom<usize>
        + Unsigned,
    usize: From<I>,
{
    let offset = 8 + mem::size_of::<T>();
    if data.len() <= offset {
        return Err(IndexedMerkleTreeError::ConcurrentMerkleTree(
            ConcurrentMerkleTreeError::BufferSize(offset, data.len()),
        ));
    }
    IndexedMerkleTreeCopy::from_bytes_copy(&data[offset..])
}

/// Parse HashSet from raw queue account data bytes
///
/// # Safety
/// Same safety requirements as `get_hash_set`.
pub unsafe fn parse_hash_set_from_bytes<T>(
    data: &[u8],
) -> Result<HashSet, light_hash_set::HashSetError> {
    let offset = 8 + mem::size_of::<T>();
    if data.len() <= offset {
        return Err(light_hash_set::HashSetError::BufferSize(offset, data.len()));
    }
    let mut data_copy = data[offset..].to_vec();
    HashSet::from_bytes_copy(&mut data_copy)
}
