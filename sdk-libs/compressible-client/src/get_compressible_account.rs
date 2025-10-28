use light_client::{
    indexer::{Indexer, TreeInfo},
    rpc::{Rpc, RpcError},
};
use light_sdk::address::v1::derive_address;
use solana_pubkey::Pubkey;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompressibleAccountError {
    #[error("RPC error: {0}")]
    Rpc(#[from] RpcError),

    #[error("Indexer error: {0}")]
    Indexer(#[from] light_client::indexer::IndexerError),

    #[error("Compressed account has no data")]
    NoData,

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[cfg(feature = "anchor")]
    #[error("Anchor deserialization error: {0}")]
    AnchorDeserialization(#[from] anchor_lang::error::Error),

    #[error("Borsh deserialization error: {0}")]
    BorshDeserialization(#[from] std::io::Error),
}

/// Fetch account data from either compressed or on-chain storage. Returns unified.
///
/// This function first checks if the account exists on-chain. If not found,
/// it derives the compressed address and fetches from compressed storage.
///
/// # Arguments
///
/// * `address` - The account address (PDA or regular address)
/// * `program_id` - The program that owns the account
/// * `address_tree_info` - The address tree information for compressed accounts
/// * `rpc` - An RPC client implementing both `Rpc` and `Indexer` traits
///
/// # Returns
///
/// Returns the account data as bytes, including the discriminator if present.
///
/// # Example
///
/// ```no_run
/// use light_compressible_client::account_fetcher::get_compressible_account_data;
/// use light_client::{
///     indexer::TreeInfo,
///     rpc::{LightClient, LightClientConfig, Rpc},
/// };
/// use solana_pubkey::Pubkey;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut rpc = LightClient::new(LightClientConfig::local()).await?;
///     
///     let address = Pubkey::new_unique();
///     let program_id = Pubkey::new_unique();
///     let address_tree_info = rpc.get_address_tree_v1();
///     
///     let account_data = get_compressible_account_data(
///         &address,
///         &program_id,
///         &address_tree_info,
///         &mut rpc,
///     ).await?;
///     
///     Ok(())
/// }
/// ```
pub async fn get_compressible_account_data<R>(
    address: &Pubkey,
    program_id: &Pubkey,
    address_tree_info: &TreeInfo,
    rpc: &mut R,
) -> Result<Vec<u8>, CompressibleAccountError>
where
    R: Rpc + Indexer,
{
    // First check if account exists on-chain
    if let Ok(Some(onchain_account)) = rpc.get_account(*address).await {
        return Ok(onchain_account.data);
    }

    // If not on-chain, check compressed storage
    // Derive the compressed address using the account address as seed
    let (compressed_address, _) =
        derive_address(&[&address.to_bytes()], &address_tree_info.tree, program_id);

    let compressed_account = rpc
        .get_compressed_account(compressed_address, None)
        .await?
        .value;

    let account_data = compressed_account
        .data
        .as_ref()
        .ok_or(CompressibleAccountError::NoData)?;

    // Combine discriminator and data
    let mut data_slice =
        Vec::with_capacity(account_data.discriminator.len() + account_data.data.len());
    data_slice.extend_from_slice(&account_data.discriminator);
    data_slice.extend_from_slice(&account_data.data);

    Ok(data_slice)
}

#[cfg(feature = "anchor")]
/// Fetch and deserialize a compressible account using Anchor.
///
/// This function combines fetching from either compressed or on-chain storage
/// with Anchor deserialization.
///
/// # Arguments
///
/// * `address` - The account address (PDA or regular address)
/// * `program_id` - The program that owns the account
/// * `address_tree_info` - The address tree information for compressed accounts
/// * `rpc` - An RPC client implementing both `Rpc` and `Indexer` traits
///
/// # Type Parameters
///
/// * `T` - The account type implementing `AccountDeserialize`
///
/// # Example
///
/// ```no_run
/// use light_compressible_client::account_fetcher::get_compressible_account;
/// use light_client::{
///     indexer::TreeInfo,
///     rpc::{LightClient, LightClientConfig, Rpc},
/// };
/// use solana_pubkey::Pubkey;
/// use anchor_lang::AccountDeserialize;
///
/// #[derive(AccountDeserialize)]
/// struct MyAccount {
///     pub data: u64,
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut rpc = LightClient::new(LightClientConfig::local()).await?;
///     
///     let address = Pubkey::new_unique();
///     let program_id = Pubkey::new_unique();
///     let address_tree_info = rpc.get_address_tree_v1();
///     
///     let account: MyAccount = get_compressible_account(
///         &address,
///         &program_id,
///         &address_tree_info,
///         &mut rpc,
///     ).await?;
///     
///     Ok(())
/// }
/// ```
pub async fn get_compressible_account<T, R>(
    address: &Pubkey,
    program_id: &Pubkey,
    address_tree_info: &TreeInfo,
    rpc: &mut R,
) -> Result<T, CompressibleAccountError>
where
    T: anchor_lang::AccountDeserialize,
    R: Rpc + Indexer,
{
    let data = get_compressible_account_data(address, program_id, address_tree_info, rpc).await?;

    T::try_deserialize(&mut data.as_slice())
        .map_err(CompressibleAccountError::AnchorDeserialization)
}

#[cfg(feature = "anchor")]
/// Deserialize an on-chain account using Anchor.
///
/// This is a utility function that deserializes an already fetched account.
pub fn deserialize_anchor_account<T>(
    account: &solana_account::Account,
) -> Result<T, CompressibleAccountError>
where
    T: anchor_lang::AccountDeserialize,
{
    T::try_deserialize(&mut &account.data[..])
        .map_err(CompressibleAccountError::AnchorDeserialization)
}