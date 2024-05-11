use std::collections::LinkedList;
use account_compression::StateMerkleTreeAccount;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use crate::errors::CrankError;
use anchor_lang::AccountDeserialize;

const INVALID_MT_PUBKEY: &str = "11111111111111111111111111111111";

pub fn reindex_and_store(merkle_tree_pubkey: &Pubkey, server_url: &str) -> Result<(), CrankError> {
    match reindex(merkle_tree_pubkey, server_url) {
        Ok(list) => {
            println!("Indexed {} merkle trees", list.len());
            serialize_indexed_mt(list)?;
        },
        Err(e) => {
            println!("Error: {}", e);
            return Err(e);
        }
    }
    Ok(())  
}

fn serialize_indexed_mt(list: LinkedList<(Pubkey, Pubkey)>) -> Result<(), CrankError> {
    let serialized = bincode::serialize(&list)?;
    std::fs::write("index.bin", serialized)?;
    Ok(())
}

fn reindex(merkle_tree_pubkey: &Pubkey, server_url: &str) -> Result<LinkedList<(Pubkey, Pubkey)>, CrankError> {
    let client = RpcClient::new(server_url);
    let mut list = LinkedList::new();

    let mut current_merkle_tree_pubkey = *merkle_tree_pubkey;
    loop {
        println!("merkle_tree_pubkey: {:?}", current_merkle_tree_pubkey);
        let nullifier_queue_pubkey = get_nullifier_queue_pubkey(&current_merkle_tree_pubkey, &client)?;
        println!("nullifier_queue_pubkey: {:?}", nullifier_queue_pubkey);
        list.push_back((current_merkle_tree_pubkey, nullifier_queue_pubkey));

        match next_merkle_tree_pubkey(&current_merkle_tree_pubkey, &client) {
            Ok(next_merkle_tree_pubkey) => {
                if next_merkle_tree_pubkey.to_string() == INVALID_MT_PUBKEY {
                    break;
                }
                current_merkle_tree_pubkey = next_merkle_tree_pubkey;
            }
            Err(_) => {
                break;
            } 
        }
    }

    Ok(list)
}


pub fn merkle_tree_account(
    merkle_tree_pubkey: &Pubkey,
    client: &RpcClient,
) -> Result<StateMerkleTreeAccount, CrankError> {
    let data: &[u8] = &client.get_account_data(merkle_tree_pubkey)?;
    let mut data_ref = &data[..];
    Ok(StateMerkleTreeAccount::try_deserialize(&mut data_ref)?)
}

pub fn next_merkle_tree_pubkey(
    merkle_tree_pubkey: &Pubkey,
    client: &RpcClient,
) -> Result<Pubkey, CrankError> {    
    let merkle_tree_account = merkle_tree_account(merkle_tree_pubkey, client)?;
    Ok(merkle_tree_account.next_merkle_tree)
}

pub fn get_nullifier_queue_pubkey(
    merkle_tree_pubkey: &Pubkey,
    client: &RpcClient,
) -> Result<Pubkey, CrankError> {
    let merkle_tree_account = merkle_tree_account(merkle_tree_pubkey, client)?;
    let nullifier_queue_pubkey = merkle_tree_account.associated_queue;
    Ok(nullifier_queue_pubkey)
}
