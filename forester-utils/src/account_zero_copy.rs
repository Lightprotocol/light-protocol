use std::{fmt, marker::PhantomData, mem, pin::Pin};

use account_compression::processor::initialize_address_merkle_tree::Pubkey;
use light_client::rpc::Rpc;
use light_concurrent_merkle_tree::copy::ConcurrentMerkleTreeCopy;
use light_hash_set::HashSet;
use light_hasher::Hasher;
use light_indexed_merkle_tree::copy::IndexedMerkleTreeCopy;
use num_traits::{CheckedAdd, CheckedSub, ToBytes, Unsigned};
use solana_sdk::account::Account;

#[derive(Debug, Clone)]
pub struct AccountZeroCopy<'a, T> {
    pub account: Pin<Box<Account>>,
    deserialized: *const T,
    _phantom_data: PhantomData<&'a T>,
}

impl<'a, T> AccountZeroCopy<'a, T> {
    pub async fn new<R: Rpc>(rpc: &mut R, address: Pubkey) -> AccountZeroCopy<'a, T> {
        let account = Box::pin(rpc.get_account(address).await.unwrap().unwrap());
        let deserialized = account.data[8..].as_ptr() as *const T;

        Self {
            account,
            deserialized,
            _phantom_data: PhantomData,
        }
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
pub async unsafe fn get_hash_set<T, R: Rpc>(rpc: &mut R, pubkey: Pubkey) -> HashSet {
    let mut data = rpc.get_account(pubkey).await.unwrap().unwrap().data.clone();

    HashSet::from_bytes_copy(&mut data[8 + mem::size_of::<T>()..]).unwrap()
}

/// Fetches the given account, then copies and serializes it as a
/// `ConcurrentMerkleTree`.
pub async fn get_concurrent_merkle_tree<T, R, H, const HEIGHT: usize>(
    rpc: &mut R,
    pubkey: Pubkey,
) -> ConcurrentMerkleTreeCopy<H, HEIGHT>
where
    R: Rpc,
    H: Hasher,
{
    let account = rpc.get_account(pubkey).await.unwrap().unwrap();

    ConcurrentMerkleTreeCopy::from_bytes_copy(&account.data[8 + mem::size_of::<T>()..]).unwrap()
}
// TODO: do discriminator check
/// Fetches the given account, then copies and serializes it as an
/// `IndexedMerkleTree`.
pub async fn get_indexed_merkle_tree<T, R, H, I, const HEIGHT: usize, const NET_HEIGHT: usize>(
    rpc: &mut R,
    pubkey: Pubkey,
) -> IndexedMerkleTreeCopy<H, I, HEIGHT, NET_HEIGHT>
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
    let account = rpc.get_account(pubkey).await.unwrap().unwrap();

    IndexedMerkleTreeCopy::from_bytes_copy(&account.data[8 + mem::size_of::<T>()..]).unwrap()
}
