use light_client::{
    indexer::{Indexer, TreeInfo},
    rpc::{Rpc, RpcError},
};
use light_sdk::address::v1::derive_address;
use solana_account::Account;
use solana_pubkey::Pubkey;
use thiserror::Error;

#[cfg(not(feature = "anchor"))]
use crate::AnchorDeserialize;

#[derive(Debug, Error)]
pub enum CompressibleAccountError {
    #[error("RPC error: {0}")]
    Rpc(#[from] RpcError),

    #[error("Indexer error: {0}")]
    Indexer(#[from] light_client::indexer::IndexerError),

    #[error("Compressed account has no data")]
    NoData,

    #[cfg(feature = "anchor")]
    #[error("Anchor deserialization error: {0}")]
    AnchorDeserialization(#[from] anchor_lang::error::Error),

    #[error("Borsh deserialization error: {0}")]
    BorshDeserialization(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct MerkleContext {
    pub tree_info: TreeInfo,
    pub hash: [u8; 32],
    pub leaf_index: u32,
    pub prove_by_index: bool,
}

#[derive(Debug, Clone)]
pub struct AccountInfoInterface {
    pub account_info: Account,
    pub is_compressed: bool,
    pub merkle_context: Option<MerkleContext>,
}

/// Get account info with unified interface.
///
/// If the account is cold, returns additional metadata for loading it to hot
/// state.
pub async fn get_account_info_interface<R>(
    address: &Pubkey,
    program_id: &Pubkey,
    address_tree_info: &TreeInfo,
    rpc: &mut R,
) -> Result<Option<AccountInfoInterface>, CompressibleAccountError>
where
    R: Rpc + Indexer,
{
    let (compressed_address, _) =
        derive_address(&[&address.to_bytes()], &address_tree_info.tree, program_id);

    let onchain_result = rpc.get_account(*address).await;
    let compressed_result = rpc.get_compressed_account(compressed_address, None).await;

    let onchain_account = onchain_result.ok().flatten();
    let compressed_account = compressed_result.ok().and_then(|r| r.value);

    if let Some(onchain) = onchain_account {
        let merkle_context = compressed_account.as_ref().map(|ca| MerkleContext {
            tree_info: ca.tree_info,
            hash: ca.hash,
            leaf_index: ca.leaf_index,
            prove_by_index: ca.prove_by_index,
        });

        return Ok(Some(AccountInfoInterface {
            account_info: onchain,
            is_compressed: false,
            merkle_context,
        }));
    }

    if let Some(ca) = compressed_account {
        if let Some(data) = ca.data.as_ref() {
            if !data.data.is_empty() {
                let mut account_data =
                    Vec::with_capacity(data.discriminator.len() + data.data.len());
                account_data.extend_from_slice(&data.discriminator);
                account_data.extend_from_slice(&data.data);

                let account_info = Account {
                    lamports: ca.lamports,
                    data: account_data,
                    owner: ca.owner,
                    executable: false,
                    // TODO: consider 0.
                    rent_epoch: u64::MAX,
                };

                return Ok(Some(AccountInfoInterface {
                    account_info,
                    is_compressed: true,
                    merkle_context: Some(MerkleContext {
                        tree_info: ca.tree_info,
                        hash: ca.hash,
                        leaf_index: ca.leaf_index,
                        prove_by_index: ca.prove_by_index,
                    }),
                }));
            }
        }
    }

    Ok(None)
}

#[cfg(feature = "anchor")]
#[allow(clippy::result_large_err)]
pub fn deserialize_account<T>(account: &AccountInfoInterface) -> Result<T, CompressibleAccountError>
where
    T: anchor_lang::AccountDeserialize,
{
    let data = &account.account_info.data;
    T::try_deserialize(&mut &data[..]).map_err(CompressibleAccountError::AnchorDeserialization)
}

#[cfg(not(feature = "anchor"))]
#[allow(clippy::result_large_err)]
pub fn deserialize_account<T>(account: &AccountInfoInterface) -> Result<T, CompressibleAccountError>
where
    T: AnchorDeserialize,
{
    let data = &account.account_info.data;
    if data.len() < 8 {
        return Err(CompressibleAccountError::BorshDeserialization(
            std::io::Error::new(std::io::ErrorKind::InvalidData, "Account data too short"),
        ));
    }
    T::deserialize(&mut &data[8..]).map_err(CompressibleAccountError::BorshDeserialization)
}

#[cfg(feature = "anchor")]
/// Get and parse account with anchor discriminator.
#[allow(clippy::result_large_err)]
pub async fn get_anchor_account<T, R>(
    address: &Pubkey,
    program_id: &Pubkey,
    address_tree_info: &TreeInfo,
    rpc: &mut R,
) -> Result<T, CompressibleAccountError>
where
    T: anchor_lang::AccountDeserialize,
    R: Rpc + Indexer,
{
    let account_interface = get_account_info_interface(address, program_id, address_tree_info, rpc)
        .await?
        .ok_or(CompressibleAccountError::NoData)?;

    deserialize_account::<T>(&account_interface)
}
