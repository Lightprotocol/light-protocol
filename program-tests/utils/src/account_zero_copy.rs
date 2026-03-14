use std::marker::PhantomData;

use account_compression::{AddressMerkleTreeAccount, QueueAccount, StateMerkleTreeAccount};
use anchor_lang::AccountDeserialize;
use borsh::BorshDeserialize;
use forester_utils::account_zero_copy::AccountZeroCopyError;
use light_batched_merkle_tree::{
    merkle_tree_metadata::BatchedMerkleTreeMetadata, queue::BatchedQueueMetadata,
};
use light_client::rpc::Rpc;
use solana_sdk::{account::Account, pubkey::Pubkey};

pub trait AccountZeroCopyDeserialize: Sized {
    fn deserialize_account(data: &[u8], pubkey: Pubkey) -> Result<Self, AccountZeroCopyError>;
}

fn deserialize_anchor_account<T: AccountDeserialize>(
    data: &[u8],
    pubkey: Pubkey,
) -> Result<T, AccountZeroCopyError> {
    if data.len() < 8 {
        return Err(AccountZeroCopyError::RpcError(format!(
            "Account {} data too short: {}",
            pubkey,
            data.len()
        )));
    }

    T::try_deserialize(&mut &data[8..]).map_err(|error| {
        AccountZeroCopyError::RpcError(format!(
            "Failed to deserialize account {}: {}",
            pubkey, error
        ))
    })
}

fn deserialize_borsh_account<T: BorshDeserialize>(
    data: &[u8],
    pubkey: Pubkey,
) -> Result<T, AccountZeroCopyError> {
    if data.len() < 8 {
        return Err(AccountZeroCopyError::RpcError(format!(
            "Account {} data too short: {}",
            pubkey,
            data.len()
        )));
    }

    T::try_from_slice(&data[8..]).map_err(|error| {
        AccountZeroCopyError::RpcError(format!(
            "Failed to deserialize account {}: {}",
            pubkey, error
        ))
    })
}

impl AccountZeroCopyDeserialize for AddressMerkleTreeAccount {
    fn deserialize_account(data: &[u8], pubkey: Pubkey) -> Result<Self, AccountZeroCopyError> {
        deserialize_anchor_account(data, pubkey)
    }
}

impl AccountZeroCopyDeserialize for QueueAccount {
    fn deserialize_account(data: &[u8], pubkey: Pubkey) -> Result<Self, AccountZeroCopyError> {
        deserialize_anchor_account(data, pubkey)
    }
}

impl AccountZeroCopyDeserialize for StateMerkleTreeAccount {
    fn deserialize_account(data: &[u8], pubkey: Pubkey) -> Result<Self, AccountZeroCopyError> {
        deserialize_anchor_account(data, pubkey)
    }
}

impl AccountZeroCopyDeserialize for BatchedMerkleTreeMetadata {
    fn deserialize_account(data: &[u8], pubkey: Pubkey) -> Result<Self, AccountZeroCopyError> {
        deserialize_borsh_account(data, pubkey)
    }
}

impl AccountZeroCopyDeserialize for BatchedQueueMetadata {
    fn deserialize_account(data: &[u8], pubkey: Pubkey) -> Result<Self, AccountZeroCopyError> {
        deserialize_borsh_account(data, pubkey)
    }
}

pub struct AccountZeroCopy<T> {
    pub account: Account,
    pub pubkey: Pubkey,
    _marker: PhantomData<T>,
}

impl<T> AccountZeroCopy<T> {
    pub async fn new<R: Rpc>(rpc: &mut R, pubkey: Pubkey) -> Result<Self, AccountZeroCopyError> {
        let account = rpc
            .get_account(pubkey)
            .await
            .map_err(|error| AccountZeroCopyError::RpcError(error.to_string()))?
            .ok_or(AccountZeroCopyError::AccountNotFound(pubkey))?;

        Ok(Self {
            account,
            pubkey,
            _marker: PhantomData,
        })
    }
}

impl<T: AccountZeroCopyDeserialize> AccountZeroCopy<T> {
    pub fn try_deserialized(&self) -> Result<T, AccountZeroCopyError> {
        T::deserialize_account(&self.account.data, self.pubkey)
    }
}
