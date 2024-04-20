use std::mem;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use account_compression::{AccountDeserialize, IndexedArrayAccount, Pubkey, StateMerkleTreeAccount};
use light_hash_set::HashSet;
use crate::indexer::get_photon_proof;

pub fn nullify_compressed_accounts(
    indexed_array_pubkey: &Pubkey,
    merkle_tree_pubkey: &Pubkey,
    payer_keypair: &Keypair,
    client: &RpcClient,
) {
    let mut indexed_array_account = client.get_account(indexed_array_pubkey).unwrap();
    let indexed_array: HashSet<u16> = unsafe {
        HashSet::from_bytes_copy(&mut indexed_array_account.data[8 + mem::size_of::<IndexedArrayAccount>()..]).unwrap()
    };
    let mut data: &[u8] = &client.get_account_data(merkle_tree_pubkey).unwrap();
    let merkle_tree_account: StateMerkleTreeAccount = StateMerkleTreeAccount::try_deserialize(&mut data).unwrap();

    let merkle_tree = merkle_tree_account.copy_merkle_tree_boxed().unwrap();
    let change_log_index = merkle_tree.current_changelog_index as u64;
    println!("Merkle tree change_log_index: {:?}", change_log_index);
    let mut compressed_accounts_to_nullify = Vec::new();
    for (i, element) in indexed_array.iter() {
        if element.sequence_number().is_none() {
            compressed_accounts_to_nullify.push((i, element.value_bytes()));
        }
    }


    for (index_in_indexed_array, compressed_account) in compressed_accounts_to_nullify.iter() {
        let proof: Vec<[u8; 32]> = get_photon_proof(compressed_account);
        let instructions = [
            account_compression::nullify_leaves::sdk_nullify::create_nullify_instruction(
                vec![change_log_index].as_slice(),
                vec![(*index_in_indexed_array) as u16].as_slice(),
                vec![0u64].as_slice(),
                vec![proof].as_slice(),
                &payer_keypair.pubkey(),
                merkle_tree_pubkey,
                indexed_array_pubkey,
            ),
        ];
        let latest_blockhash = client.get_latest_blockhash().unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer_keypair.pubkey()),
            &[&payer_keypair],
            latest_blockhash,
        );
        let tx_result = client.send_and_confirm_transaction(&transaction).unwrap();
        println!("Transaction signature: {:?}", tx_result);
    }
}