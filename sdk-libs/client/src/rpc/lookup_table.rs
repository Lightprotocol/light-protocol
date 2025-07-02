pub use solana_address_lookup_table_interface::{
    error, instruction, program, state::AddressLookupTable,
};
use solana_message::AddressLookupTableAccount;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;

use crate::rpc::errors::RpcError;

/// Gets a lookup table account state from the network.
///
/// # Arguments
///
/// * `client` - The RPC client to use to get the lookup table account state.
/// * `lookup_table_address` - The address of the lookup table account to get.
///
/// # Returns
///
/// * `AddressLookupTableAccount` - The lookup table account state.
pub fn load_lookup_table(
    client: &RpcClient,
    lookup_table_address: &Pubkey,
) -> Result<AddressLookupTableAccount, RpcError> {
    let raw_account = client.get_account(lookup_table_address)?;
    let address_lookup_table = AddressLookupTable::deserialize(&raw_account.data).map_err(|e| {
        RpcError::CustomError(format!("Failed to deserialize AddressLookupTable: {e:?}"))
    })?;
    let address_lookup_table_account = AddressLookupTableAccount {
        key: lookup_table_address.to_bytes().into(),
        addresses: address_lookup_table
            .addresses
            .into_iter()
            .map(|p| p.to_bytes().into())
            .collect(),
    };
    Ok(address_lookup_table_account)
}
