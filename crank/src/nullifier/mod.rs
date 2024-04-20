mod nullify;
mod subscribe;

pub use nullify::nullify_compressed_accounts;
pub use subscribe::subscribe_nullify;

// fn get_nullifier_info(nullifier: &str, client: &RpcClient) {
//     let nullifier_pubkey = Pubkey::from_str(nullifier).unwrap();
//     let mut data: &[u8] = &client.get_account_data(&nullifier_pubkey).unwrap();
//     let indexed_account: IndexedArrayAccount =
//         IndexedArrayAccount::try_deserialize(&mut data).unwrap();
//     println!(
//         "Nullifier account associated merkle tree pubkey: {:?}",
//         indexed_account.associated_merkle_tree
//     );
// }

